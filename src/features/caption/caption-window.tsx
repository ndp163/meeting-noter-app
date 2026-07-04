import { useEffect, useLayoutEffect, useRef, useState } from "react";
import {
  getCurrentWindow,
  currentMonitor,
  LogicalSize,
  LogicalPosition,
} from "@tauri-apps/api/window";
import {
  listenToTranscription,
  type TranscriptionSource,
  type TranscriptionEventPayload,
} from "@/services/transcription";
import { listenToTranslation } from "@/services/translation";

/**
 * Standalone UI rendered in the always-on-top `caption` overlay window.
 *
 * It subscribes directly to `transcription://chunk` (broadcast to every window)
 * and shows a merged, chronological caption feed — the most recent few lines,
 * each tagged by speaker. Within a line, the finalized (committed) text renders
 * solid and the still-interim tail renders dimmed + italic, so it's clear what
 * ASR has locked in versus what may still change.
 *
 * The window resizes itself to hug this content, so the card is always exactly
 * as tall as the text (capped), anchored bottom-centre of the current monitor.
 */

/** Below this ASR confidence the committed text renders dimmed. */
const LOW_CONFIDENCE = 0.5;
/** How many lines are visible at once (older ones scroll off the top). */
const MAX_VISIBLE = 3;
/** Cap on retained lines (memory bound). */
const MAX_KEEP = 12;
/** Caption text size (px). */
const FONT_PX = 18;
/** Gap (logical px) from the screen edge. */
const EDGE_MARGIN = 72;
/** Max caption width (logical px) before text wraps. */
const MAX_WIDTH = 880;
/** Max caption height (logical px); older lines clip off the top. */
const MAX_HEIGHT = 260;

interface FeedItem {
  id: string;
  source: TranscriptionSource;
  /** Offset (sec) of this line in the recording — matches the translation
   *  event's `start_sec`, used to attach the translated text. */
  startSec: number;
  /** Finalized text for this sentence (solid). */
  committed: string;
  /** Interim, still-changing tail (dimmed + italic). Empty once finalized. */
  partial: string;
  /** On-device translation of this line, when Translation is enabled. */
  translation?: string;
  lowConfidence: boolean;
}

/** start_sec values match to this tolerance (seconds). */
const OFFSET_EPS = 0.05;

const join = (a: string, b: string) => (a ? `${a} ${b}` : b);

export const CaptionWindow = () => {
  const [feed, setFeed] = useState<FeedItem[]>([]);
  // In-progress line id per source; null once its sentence finalizes.
  const currentId = useRef<Record<string, string | null>>({});
  const counter = useRef(0);

  useEffect(() => {
    let unlisten: (() => void) | null = null;

    listenToTranscription((payload: TranscriptionEventPayload) => {
      if (!payload.text.trim()) return;
      const s = payload.source;
      const existing = currentId.current[s] ?? null;
      const isNew = existing === null;
      const id = isNew ? `c${++counter.current}` : existing;
      // Next partial after a finalized sentence starts a fresh line.
      currentId.current[s] = payload.is_sentence_final ? null : id;

      setFeed((prev) => {
        const cur = isNew ? undefined : prev.find((i) => i.id === id);
        const prevCommitted = cur?.committed ?? "";
        const lowConfidence = payload.confidence < LOW_CONFIDENCE;

        let committed: string;
        let partial: string;
        if (payload.is_result_final) {
          // A segment (or whole sentence) locked in — fold it into committed.
          committed = join(prevCommitted, payload.text);
          partial = "";
        } else {
          // Live streaming guess — the interim tail after committed text.
          committed = prevCommitted;
          partial = payload.text;
        }

        if (isNew) {
          const item: FeedItem = {
            id,
            source: s,
            startSec: payload.start_sec,
            committed,
            partial,
            lowConfidence,
          };
          return [...prev, item].slice(-MAX_KEEP);
        }
        return prev.map((i) =>
          i.id === id ? { ...i, committed, partial, lowConfidence } : i,
        );
      });
    }).then((release) => {
      unlisten = release;
    });

    return () => {
      unlisten?.();
    };
  }, []);

  // Attach on-device translations (only emitted when Translation is enabled in
  // Settings). Match to a line by source + start_sec.
  useEffect(() => {
    let unlisten: (() => void) | null = null;
    listenToTranslation((payload) => {
      if (!payload.text.trim()) return;
      setFeed((prev) =>
        prev.map((i) =>
          i.source === payload.source &&
          Math.abs(i.startSec - payload.start_sec) < OFFSET_EPS
            ? { ...i, translation: payload.text }
            : i,
        ),
      );
    }).then((release) => {
      unlisten = release;
    });
    return () => {
      unlisten?.();
    };
  }, []);

  // Resize the window to hug the caption card. It grows/shrinks around a fixed
  // anchor (bottom-centre point) so it never jumps back to the start once the
  // user drags it — the anchor tracks wherever the window currently sits.
  const barRef = useRef<HTMLDivElement>(null);
  const lastSize = useRef({ w: 0, h: 0 });
  const scale = useRef(1);
  // Logical bottom-centre point the card is pinned to; null until first layout.
  const anchor = useRef<{ centerX: number; bottom: number } | null>(null);
  // True while we're programmatically resizing/repositioning, so the resulting
  // move events don't get mistaken for a user drag.
  const busy = useRef(false);
  useLayoutEffect(() => {
    const el = barRef.current;
    if (!el) return;
    const win = getCurrentWindow();
    let unmoved: (() => void) | null = null;

    const resize = async () => {
      const rect = el.getBoundingClientRect();
      const w = Math.ceil(rect.width);
      const h = Math.ceil(rect.height);
      if (
        Math.abs(w - lastSize.current.w) < 2 &&
        Math.abs(h - lastSize.current.h) < 2
      ) {
        return;
      }
      lastSize.current = { w, h };
      busy.current = true;
      await win.setSize(new LogicalSize(w, h));

      // Seed the anchor at the monitor's bottom-centre on first layout.
      if (!anchor.current) {
        const mon = await currentMonitor();
        if (mon) {
          scale.current = mon.scaleFactor;
          const s = mon.scaleFactor;
          anchor.current = {
            centerX: mon.position.x / s + mon.size.width / s / 2,
            bottom: mon.position.y / s + mon.size.height / s - EDGE_MARGIN,
          };
        }
      }
      if (anchor.current) {
        const x = anchor.current.centerX - w / 2;
        const y = anchor.current.bottom - h;
        await win.setPosition(new LogicalPosition(x, y));
      }
      busy.current = false;
    };

    // When the user drags the window, re-derive the anchor from its new spot so
    // subsequent resizes grow around there instead of the original centre.
    void win.onMoved(({ payload }) => {
      if (busy.current) return;
      const s = scale.current;
      const { w, h } = lastSize.current;
      anchor.current = { centerX: payload.x / s + w / 2, bottom: payload.y / s + h };
    }).then((un) => {
      unmoved = un;
    });

    void resize();
    const ro = new ResizeObserver(() => void resize());
    ro.observe(el);
    return () => {
      ro.disconnect();
      unmoved?.();
    };
  }, []);

  const visible = feed.slice(-MAX_VISIBLE);

  return (
    // The card IS the window (auto-sized to this element). Transparent corners
    // come from the transparent window; drag anywhere via the drag region.
    // justify-end + overflow-hidden keeps the newest line pinned to the bottom
    // once the height cap is hit.
    <div
      ref={barRef}
      data-tauri-drag-region
      className="inline-flex w-max cursor-grab select-none flex-col justify-end gap-1 overflow-hidden rounded-[var(--ds-radius)] px-5 py-3 shadow-[var(--ds-shadow-lg)] active:cursor-grabbing"
      style={{
        background: "var(--ds-caption-bg)",
        maxWidth: MAX_WIDTH,
        maxHeight: MAX_HEIGHT,
      }}
    >
      <style>{`@keyframes cap-in{from{opacity:0;transform:translateY(4px)}to{opacity:1;transform:none}}`}</style>
      {visible.length === 0 ? (
        <div className="pointer-events-none flex items-center gap-2.5 py-0.5">
          <span
            className="h-2 w-2 rounded-full animate-pulse"
            style={{ background: "var(--ds-caption-mic)" }}
          />
          <span
            className="italic"
            style={{ color: "var(--ds-caption-text-2)", fontSize: FONT_PX - 3 }}
          >
            Listening…
          </span>
        </div>
      ) : (
        visible.map((line, idx) => {
          // Older lines fade toward the top; newest is full strength.
          const age = visible.length - 1 - idx;
          const committedColor =
            line.lowConfidence || age > 0
              ? "var(--ds-caption-text-2)"
              : "var(--ds-caption-text)";
          return (
            <p
              key={line.id}
              className="pointer-events-none leading-snug break-words"
              style={{
                fontSize: FONT_PX,
                opacity: age === 0 ? 1 : age === 1 ? 0.7 : 0.45,
                animation: "cap-in 160ms ease-out",
              }}
            >
              <span
                className="mr-2 align-baseline font-semibold"
                style={{
                  fontSize: FONT_PX,
                  color:
                    line.source === "mic"
                      ? "var(--ds-caption-mic)"
                      : "var(--ds-caption-speaker)",
                }}
              >
                {line.source === "mic" ? "You" : "Speaker"}
              </span>
              {line.translation ? (
                // Translation replaces the original once it lands (it arrives as
                // whole text, so there's no committed/partial split to show).
                <span style={{ color: committedColor }}>{line.translation}</span>
              ) : (
                <>
                  {line.committed && (
                    <span style={{ color: committedColor }}>
                      {line.committed}
                    </span>
                  )}
                  {line.partial && (
                    <span
                      style={{
                        color: "var(--ds-caption-text-2)",
                        fontStyle: "italic",
                      }}
                    >
                      {line.committed ? ` ${line.partial}` : line.partial}
                    </span>
                  )}
                </>
              )}
            </p>
          );
        })
      )}
    </div>
  );
};
