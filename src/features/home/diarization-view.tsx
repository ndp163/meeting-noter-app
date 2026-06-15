import { useState } from "react";
import { Pencil } from "lucide-react";
import { cn } from "@/lib/utils";
import type { DiarizedSegment } from "@/types/meeting";

interface DiarizationViewProps {
  segments: DiarizedSegment[] | undefined;
  isLoading: boolean;
  error?: string;
  canRun: boolean;
  onRun: () => void;
  onRenameSpeaker: (speakerId: string, label: string) => void;
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
      <div className="flex flex-col gap-3">
        <p className="text-sm text-custom-red">{error}</p>
        <button
          onClick={onRun}
          className="self-start rounded-full border border-custom-text-primary px-4 py-1.5 text-sm text-custom-text-primary hover:bg-custom-bg-secondary"
        >
          Try again
        </button>
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
      <div className="flex flex-col gap-3">
        <p className="text-sm text-custom-text-secondary">
          Identify who spoke when. Transcribes the whole recording and groups it
          by speaker. You can rename each speaker afterwards.
        </p>
        <button
          onClick={onRun}
          className="self-start rounded-full border border-custom-text-primary px-4 py-1.5 text-sm text-custom-text-primary hover:bg-custom-bg-secondary"
        >
          Identify speakers
        </button>
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
    <div className="flex flex-col gap-5">
      {segments.map((segment, index) => {
        const isYou = segment.speakerId === "you";
        return (
          <div key={`${segment.speakerId}-${index}`} className="flex flex-col gap-2.5">
            <div className="flex gap-2.5 items-center">
              <div className="flex items-center justify-center border-b border-custom-primary pb-0.5">
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
                    className="text-base bg-transparent outline-none border-b border-custom-text-primary w-28"
                  />
                ) : (
                  <button
                    onClick={() => {
                      setEditing(index);
                      setDraft(segment.label);
                    }}
                    title="Click to rename"
                    className={cn(
                      "group flex items-center gap-1.5 text-base hover:underline",
                      isYou
                        ? "text-custom-text-highlight"
                        : "text-custom-text-primary"
                    )}
                  >
                    {segment.label}
                    <Pencil className="w-3.5 h-3.5 text-custom-text-secondary opacity-0 group-hover:opacity-100" />
                  </button>
                )}
              </div>
              <p className="text-base text-custom-text-secondary">
                {formatTime(segment.start)}
              </p>
            </div>
            <p className="text-base text-custom-text-primary whitespace-pre-wrap">
              {segment.text}
            </p>
          </div>
        );
      })}
    </div>
  );
};
