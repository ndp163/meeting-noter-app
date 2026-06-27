import { useState } from "react";
import { Pencil } from "lucide-react";
import { Avatar, Button } from "@/design-system";
import type { DiarizedSegment } from "@/types/meeting";

interface DiarizationViewProps {
  segments: DiarizedSegment[] | undefined;
  isLoading: boolean;
  error?: string;
  canRun: boolean;
  onRun: () => void;
  onRenameSpeaker: (speakerId: string, label: string) => void;
  onSeek: (seconds: number) => void;
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
  error,
  canRun,
  onRun,
  onRenameSpeaker,
  onSeek,
}: DiarizationViewProps) => {
  const [editing, setEditing] = useState<number | null>(null);
  const [draft, setDraft] = useState("");

  if (isLoading) {
    return (
      <div className="text-custom-text-secondary text-sm">
        Transcribing and identifying speakers…
      </div>
    );
  }

  if (error) {
    return (
      <div className="flex flex-col gap-3 items-start">
        <p className="text-sm text-custom-red">{error}</p>
        <Button variant="ghost" pill onClick={onRun}>
          Try again
        </Button>
      </div>
    );
  }

  if (!segments || segments.length === 0) {
    if (!canRun) {
      return (
        <div className="text-custom-text-secondary text-sm">
          Diarization is available after a recording finishes.
        </div>
      );
    }
    return (
      <div className="flex flex-col gap-3 items-start">
        <p className="text-sm text-custom-text-secondary">
          Identify who spoke when. Transcribes the whole recording and groups it
          by speaker. You can rename each speaker afterwards.
        </p>
        <Button variant="ghost" pill onClick={onRun}>
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
                    value={draft}
                    onChange={(e) => setDraft(e.target.value)}
                    onBlur={() => commitEdit(segment.speakerId)}
                    onKeyDown={(e) => {
                      if (e.key === "Enter") commitEdit(segment.speakerId);
                      if (e.key === "Escape") setEditing(null);
                    }}
                    className="bg-transparent outline-none border-b w-32"
                    style={{ borderColor: "var(--ds-border-2)", color: "var(--ds-text)" }}
                  />
                ) : (
                  <>
                    {segment.label}
                    <span className="ds-seg__time">{formatTime(segment.start)}</span>
                    <span
                      className="ds-seg__edit"
                      title="Rename speaker"
                      onClick={() => {
                        setEditing(index);
                        setDraft(segment.label);
                      }}
                    >
                      <Pencil size={13} />
                    </span>
                    <button
                      onClick={() => onSeek(segment.start)}
                      title="Jump to this moment"
                      className="ds-seg__time hover:underline"
                      style={{ background: "none", border: "none", cursor: "pointer", padding: 0 }}
                    >
                      ▸ play
                    </button>
                  </>
                )}
              </div>
              <div className="ds-seg__txt">{segment.text}</div>
            </div>
          </div>
        );
      })}
    </div>
  );
};
