import { RefObject, useEffect, useRef, useState } from "react";
import { ChevronDown, ChevronUp, Search, X } from "lucide-react";
import { IconButton } from "@/design-system";
import { useBoundStore } from "@/store";
import { findMatches, foldText } from "@/lib/search";

// CSS Custom Highlight API (WebKit 17.2+). Without it, find still counts and
// scrolls to matches, it just can't paint them.
const supportsHighlight = typeof CSS !== "undefined" && "highlights" in CSS;

/** A Range per match of `query` across the text nodes under `root`, so a match
 *  may span word-level spans. */
const collectRanges = (root: HTMLElement, query: string): Range[] => {
  const q = foldText(query.trim());
  if (!q) return [];

  const nodes: Text[] = [];
  const starts: number[] = [];
  let text = "";
  const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT);
  for (let n = walker.nextNode(); n; n = walker.nextNode()) {
    nodes.push(n as Text);
    starts.push(text.length);
    text += n.nodeValue ?? "";
  }

  // Matches come in ascending order, so the node pointer only moves forward.
  let ni = 0;
  const locate = (pos: number): [Text, number] => {
    while (ni + 1 < nodes.length && starts[ni + 1] <= pos) ni++;
    return [nodes[ni], pos - starts[ni]];
  };
  return findMatches(foldText(text), q).map((start) => {
    const range = document.createRange();
    range.setStart(...locate(start));
    range.setEnd(...locate(start + q.length));
    return range;
  });
};

interface FindBarProps {
  /** Element whose text is searched. */
  containerRef: RefObject<HTMLElement | null>;
  /** Changes when the searched content does (meeting / tab) — restarts at the
   *  first match. */
  scopeKey: string;
}

/** ⌘F find-in-meeting bar: highlights matches in the visible tab. */
export const FindBar = ({ containerRef, scopeKey }: FindBarProps) => {
  const query = useBoundStore.use.findQuery();
  const setQuery = useBoundStore.use.setFindQuery();
  const inputRef = useRef<HTMLInputElement>(null);
  const [ranges, setRanges] = useState<Range[]>([]);
  const [index, setIndex] = useState(0);
  // Scroll only when the user moves (new query / next / prev / new scope), not
  // on every live-transcript update.
  const pendingScroll = useRef(false);

  // ⌘F opens the bar, or refocuses it when already open.
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (!e.metaKey || e.key.toLowerCase() !== "f") return;
      e.preventDefault();
      setQuery(useBoundStore.getState().findQuery ?? "");
      requestAnimationFrame(() => inputRef.current?.select());
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [setQuery]);

  useEffect(() => {
    setIndex(0);
    pendingScroll.current = true;
  }, [query, scopeKey]);

  // Re-scan whenever the content changes (live transcript, tab switch, …).
  useEffect(() => {
    const root = containerRef.current;
    if (query === null || !root) {
      setRanges([]);
      return;
    }
    let frame = 0;
    const scan = () => {
      cancelAnimationFrame(frame);
      frame = requestAnimationFrame(() => setRanges(collectRanges(root, query)));
    };
    scan();
    const observer = new MutationObserver(scan);
    observer.observe(root, { childList: true, subtree: true, characterData: true });
    return () => {
      observer.disconnect();
      cancelAnimationFrame(frame);
    };
  }, [containerRef, query, scopeKey]);

  const current = ranges.length ? Math.min(index, ranges.length - 1) : -1;

  useEffect(() => {
    const range = ranges[current];
    if (supportsHighlight) {
      CSS.highlights.set("find", new Highlight(...ranges));
      if (range) CSS.highlights.set("find-current", new Highlight(range));
      else CSS.highlights.delete("find-current");
    }
    if (range && pendingScroll.current) {
      pendingScroll.current = false;
      range.startContainer.parentElement?.scrollIntoView({ block: "center" });
    }
  }, [ranges, current]);

  useEffect(
    () => () => {
      if (!supportsHighlight) return;
      CSS.highlights.delete("find");
      CSS.highlights.delete("find-current");
    },
    [],
  );

  if (query === null) return null;

  const step = (delta: number) => {
    if (!ranges.length) return;
    pendingScroll.current = true;
    setIndex((current + delta + ranges.length) % ranges.length);
  };

  return (
    <div className="flex items-center gap-1 h-10 pl-3 pr-1 shrink-0 rounded-[var(--ds-radius-sm)] border border-[var(--ds-border)] bg-[var(--ds-surface)]">
      <Search className="w-4 h-4 shrink-0 text-[var(--ds-text-3)]" />
      <input
        ref={inputRef}
        autoFocus
        value={query}
        onChange={(e) => setQuery(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter") step(e.shiftKey ? -1 : 1);
          else if (e.key === "Escape") setQuery(null);
        }}
        placeholder="Find in meeting"
        aria-label="Find in meeting"
        className="flex-1 min-w-0 px-1 text-sm bg-transparent outline-none text-[var(--ds-text)] placeholder:text-[var(--ds-text-3)]"
      />
      {query.trim() && (
        <span className="px-1 text-xs tabular-nums text-[var(--ds-text-3)]">
          {ranges.length ? `${current + 1}/${ranges.length}` : "No results"}
        </span>
      )}
      <IconButton
        icon={<ChevronUp className="w-4 h-4" />}
        label="Previous match"
        disabled={!ranges.length}
        onClick={() => step(-1)}
      />
      <IconButton
        icon={<ChevronDown className="w-4 h-4" />}
        label="Next match"
        disabled={!ranges.length}
        onClick={() => step(1)}
      />
      <IconButton
        icon={<X className="w-4 h-4" />}
        label="Close"
        onClick={() => setQuery(null)}
      />
    </div>
  );
};
