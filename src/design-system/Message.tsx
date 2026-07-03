import { clsx } from "./util";

export interface MessageProps {
  /** Speaker name / label. */
  speaker: string;
  /** Pre-formatted timestamp, e.g. "00:11". */
  timestamp: string;
  /** Transcript text. Preserves line breaks. */
  content: string;
  /** Optional translated text, shown muted beneath the body. */
  translation?: string;
  /** Render the speaker in the accent colour (the local user). */
  isUser?: boolean;
  /** Add a divider beneath the message (between consecutive messages). */
  divided?: boolean;
  /** Jump-to-moment handler; makes the timestamp and body clickable. */
  onSeek?: () => void;
  /** Highlight this line as the segment currently playing. */
  active?: boolean;
  /** Text is still interim (not finalized by ASR) — rendered dimmed/italic. */
  pending?: boolean;
}

/** A single transcript line: speaker, timestamp, body. */
export const Message = ({
  speaker,
  timestamp,
  content,
  translation,
  isUser,
  divided,
  onSeek,
  active,
  pending,
}: MessageProps) => {
  // Click anywhere on the body seeks — unless the user is selecting text to copy.
  const handleBodyClick = () => {
    if (!onSeek) return;
    if ((window.getSelection()?.toString().length ?? 0) > 0) return;
    onSeek();
  };
  return (
    <div
      className={clsx("ds-msg", divided && "ds-msg--divided", active && "ds-msg--active")}
    >
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
      <p
        className={clsx(
          "ds-msg__body",
          onSeek && "ds-msg__body--seekable",
          pending && "ds-msg__body--pending",
        )}
        onClick={onSeek ? handleBodyClick : undefined}
        role={onSeek ? "button" : undefined}
        tabIndex={onSeek ? 0 : undefined}
        aria-label={onSeek ? "Jump to this moment" : undefined}
        onKeyDown={
          onSeek
            ? (e) => {
                if (e.key === "Enter") {
                  e.preventDefault();
                  onSeek();
                }
              }
            : undefined
        }
      >
        {content}
      </p>
      {translation && <p className="ds-msg__translation">{translation}</p>}
    </div>
  );
};
