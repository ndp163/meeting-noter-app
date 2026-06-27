import { clsx } from "./util";

export interface MessageProps {
  /** Speaker name / label. */
  speaker: string;
  /** Pre-formatted timestamp, e.g. "00:11". */
  timestamp: string;
  /** Transcript text. Preserves line breaks. */
  content: string;
  /** Render the speaker in the accent colour (the local user). */
  isUser?: boolean;
  /** Add a divider beneath the message (between consecutive messages). */
  divided?: boolean;
  /** Jump-to-moment handler; makes the timestamp clickable. */
  onSeek?: () => void;
}

/** A single transcript line: speaker, timestamp, body. */
export const Message = ({
  speaker,
  timestamp,
  content,
  isUser,
  divided,
  onSeek,
}: MessageProps) => (
  <div className={clsx("ds-msg", divided && "ds-msg--divided")}>
    <div className="ds-msg__head">
      <span className={clsx("ds-msg__speaker", isUser && "ds-msg__speaker--me")}>
        {speaker}
      </span>
      {onSeek ? (
        <button className="ds-msg__ts" onClick={onSeek} title="Jump to this moment">
          {timestamp}
        </button>
      ) : (
        <span className="ds-msg__ts">{timestamp}</span>
      )}
    </div>
    <p className="ds-msg__body">{content}</p>
  </div>
);
