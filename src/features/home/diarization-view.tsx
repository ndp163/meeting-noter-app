import { useState } from "react";
import { Pencil, Play, RotateCcw, Users } from "lucide-react";
import { Avatar, Button, ProgressBar, Stepper } from "@/design-system";
import type { DiarizedSegment } from "@/types/meeting";

interface DiarizationViewProps {
  segments: DiarizedSegment[] | undefined;
  isLoading: boolean;
  /** Fraction 0–1 while running; 0 means the transcription phase (no ticks). */
  progress: number;
  error?: string;
  canRun: boolean;
  onRun: (numSpeakers?: number) => void;
  onRenameSpeaker: (speakerId: string, label: string) => void;
  onSeek: (seconds: number) => void;
  /** `"segIndex:wordIndex"` of the word under the playhead, for highlight. */
  activeWordId?: string | null;
}

const formatTime = (seconds: number) => {
  const total = Math.max(0, Math.floor(seconds));
  const mm = Math.floor(total / 60);
  const ss = total % 60;
  return `${String(mm).padStart(2, "0")}:${String(ss).padStart(2, "0")}`;
};

export const DiarizationView = ({
  segments,
  isLoading,
  progress,
  error,
  canRun,
  onRun,
  onRenameSpeaker,
  onSeek,
  activeWordId,
}: DiarizationViewProps) => {
  const [editing, setEditing] = useState<number | null>(null);
  const [draft, setDraft] = useState("");
  // Expected number of remote speakers ("you" is tracked separately from the
  // mic). null = automatic clustering.
  const [speakerCount, setSpeakerCount] = useState<number | null>(null);

  const run = () => onRun(speakerCount ?? undefined);

  if (isLoading) {
    return (
      <div className="flex flex-col gap-3">
        <p className="text-[var(--ds-text-2)] text-sm">
          Transcribing and identifying speakers…
        </p>
        <ProgressBar
          value={progress}
          indeterminate={progress === 0}
          className="max-w-72"
        />
      </div>
    );
  }

  if (error) {
    return (
      <div className="flex flex-col gap-3 items-start">
        <p className="text-sm text-[var(--ds-rec)]">{error}</p>
        <Button
          variant="ghost"
          size="sm"
          icon={<RotateCcw className="w-4 h-4" />}
          onClick={run}
        >
          Try again
        </Button>
      </div>
    );
  }

  if (!segments || segments.length === 0) {
    if (!canRun) {
      return (
        <div className="text-[var(--ds-text-2)] text-sm">
          Diarization is available after a recording finishes.
        </div>
      );
    }
    return (
      <div className="flex flex-col gap-3 items-start">
        <p className="text-sm text-[var(--ds-text-2)]">
          Identify who spoke when. Transcribes the whole recording and groups it
          by speaker. You can rename each speaker afterwards.
        </p>
        <div className="flex items-center gap-3">
          <Stepper
            aria-label="Number of speakers"
            value={speakerCount}
            onChange={setSpeakerCount}
            min={1}
            max={20}
            placeholder="Auto"
          />
          <div className="flex flex-col">
            <span className="text-sm" style={{ color: "var(--ds-text)" }}>
              Speakers on the call
            </span>
            <span className="text-xs text-[var(--ds-text-2)]">
              Besides you · leave on Auto to detect it
            </span>
          </div>
        </div>
        <Button
          variant="primary"
          size="sm"
          icon={<Users className="w-4 h-4" />}
          onClick={run}
        >
          Identify speakers
        </Button>
      </div>
    );
  }

  const commitEdit = (speakerId: string) => {
    const label = draft.trim();
    if (label) {
      onRenameSpeaker(speakerId, label);
    }
    setEditing(null);
  };

  return (
    <div className="flex flex-col">
      {segments.map((segment, index) => {
        const isYou = segment.speakerId === "you";
        const divided = index < segments.length - 1;
        return (
          <div
            key={`${segment.speakerId}-${index}`}
            className={`ds-seg${divided ? " ds-seg--divided" : ""}`}
          >
            <Avatar
              name={segment.label}
              color={isYou ? "var(--ds-accent)" : undefined}
            />
            <div style={{ minWidth: 0, flex: 1 }}>
              <div className="ds-seg__who">
                {editing === index ? (
                  <input
                    autoFocus
                    aria-label="Speaker name"
                    value={draft}
                    onChange={(e) => setDraft(e.target.value)}
                    onBlur={() => commitEdit(segment.speakerId)}
                    onKeyDown={(e) => {
                      if (e.key === "Enter") commitEdit(segment.speakerId);
                      if (e.key === "Escape") setEditing(null);
                    }}
                    className="bg-transparent outline-none border-b w-32"
                    style={{
                      borderColor: "var(--ds-border-2)",
                      color: "var(--ds-text)",
                    }}
                  />
                ) : (
                  <>
                    {segment.label}
                    <span className="ds-seg__time">
                      {formatTime(segment.start)}
                    </span>
                    <button
                      type="button"
                      className="ds-seg__edit"
                      title="Rename speaker"
                      aria-label={`Rename ${segment.label}`}
                      onClick={() => {
                        setEditing(index);
                        setDraft(segment.label);
                      }}
                    >
                      <Pencil size={13} />
                    </button>
                    <button
                      onClick={() => onSeek(segment.start)}
                      title="Jump to this moment"
                      aria-label="Play from here"
                      className="ds-seg__time inline-flex items-center gap-1 hover:underline"
                      style={{
                        background: "none",
                        border: "none",
                        cursor: "pointer",
                        padding: 0,
                      }}
                    >
                      <Play size={11} fill="currentColor" />
                      play
                    </button>
                  </>
                )}
              </div>
              {/* Word-level seek: each word carries its exact time span, so a
                  click jumps the audio right to it. The word under the
                  playhead is highlighted (segment text is the fallback for
                  meetings diarized before word timings existed). */}
              <div className="ds-seg__txt">
                {segment.words?.length
                  ? segment.words.map((word, wi) => {
                      const id = `${index}:${wi}`;
                      return (
                        <span key={wi}>
                          {wi > 0 && " "}
                          <span
                            data-word-id={id}
                            onClick={() => onSeek(word.start)}
                            title="Jump to this word"
                            className="cursor-pointer rounded-[3px] px-px -mx-px hover:bg-[var(--ds-surface-2)]"
                            style={
                              activeWordId === id
                                ? {
                                    background: "var(--ds-border-2)",
                                    color: "var(--ds-text)",
                                  }
                                : undefined
                            }
                          >
                            {word.text}
                          </span>
                        </span>
                      );
                    })
                  : segment.text}
              </div>
            </div>
          </div>
        );
      })}
    </div>
  );
};
