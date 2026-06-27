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
    style={onSeek ? { cursor: "pointer" } : undefined}
  >
    <Avatar name={speaker} color={color} />
    <div>
      <div className="ds-seg__who">
        {speaker}
        <span className="ds-seg__time">{timestamp}</span>
        {onRename && (
          <span
            className="ds-seg__edit"
            onClick={(e) => {
              e.stopPropagation();
              onRename();
            }}
            title="Rename speaker"
          >
            <Pencil size={13} />
          </span>
        )}
      </div>
      <div className="ds-seg__txt">{content}</div>
    </div>
  </div>
);
