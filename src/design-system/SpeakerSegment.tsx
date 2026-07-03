import { Pencil } from "lucide-react";
import { clsx } from "./util";
import { Avatar } from "./Avatar";

export interface SpeakerSegmentProps {
  /** Speaker name (drives the avatar + heading). */
  speaker: string;
  /** Pre-formatted timestamp, e.g. "16:45:02". */
  timestamp: string;
  /** Spoken text for this segment. */
  content: string;
  /** Override the avatar colour. */
  color?: string;
  /** Add a divider beneath the segment. */
  divided?: boolean;
  /** Show the inline rename affordance; fires when clicked. */
  onRename?: () => void;
  /** Make the row seekable. */
  onSeek?: () => void;
}

/** Diarization row: avatar, speaker + time, spoken text, optional rename. */
export const SpeakerSegment = ({
  speaker,
  timestamp,
  content,
  color,
  divided,
  onRename,
  onSeek,
}: SpeakerSegmentProps) => (
  <div
    className={clsx("ds-seg", divided && "ds-seg--divided")}
    onClick={onSeek}
    role={onSeek ? "button" : undefined}
    tabIndex={onSeek ? 0 : undefined}
    aria-label={onSeek ? `Jump to ${speaker} at ${timestamp}` : undefined}
    onKeyDown={
      onSeek
        ? (e) => {
            if (e.key === "Enter" || e.key === " ") {
              e.preventDefault();
              onSeek();
            }
          }
        : undefined
    }
    style={onSeek ? { cursor: "pointer" } : undefined}
  >
    <Avatar name={speaker} color={color} />
    <div>
      <div className="ds-seg__who">
        {speaker}
        <span className="ds-seg__time">{timestamp}</span>
        {onRename && (
          <button
            type="button"
            className="ds-seg__edit"
            onClick={(e) => {
              e.stopPropagation();
              onRename();
            }}
            title="Rename speaker"
            aria-label="Rename speaker"
          >
            <Pencil size={13} />
          </button>
        )}
      </div>
      <div className="ds-seg__txt">{content}</div>
    </div>
  </div>
);
