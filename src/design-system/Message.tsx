import { clsx } from "./util";

/** One word of `content` with its time span (seconds) in the recording. */
export interface MessageWord {
  text: string;
  start: number;
  end: number;
}

export interface MessageProps {
  /** Speaker name / label. */
  speaker: string;
  /** Pre-formatted timestamp, e.g. "00:11". */
  timestamp: string;
  /** Transcript text (finalized / committed). Preserves line breaks. */
  content: string;
  /** Interim tail still being revised by ASR — rendered dimmed + italic after
   *  the committed `content`, so it's clear which words aren't locked in yet. */
  partial?: string;
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
  /** Word-level time spans replacing `content`'s plain rendering; each word
   *  becomes a click-to-seek target when `onWordClick` is set. */
  words?: MessageWord[];
  /** Index into `words` of the word under the playhead (karaoke highlight). */
  activeWordIndex?: number;
  /** Per-word seek; receives the clicked word's start time (seconds). */
  onWordClick?: (seconds: number) => void;
}

/** A single transcript line: speaker, timestamp, body. */
export const Message = ({
  speaker,
  timestamp,
  content,
  partial,
  translation,
  isUser,
  divided,
  onSeek,
  active,
  pending,
  words,
  activeWordIndex,
  onWordClick,
}: MessageProps) => {
  // Click anywhere on the body seeks — unless the user is selecting text to copy.
  const handleBodyClick = () => {
    if (!onSeek) return;
    if ((window.getSelection()?.toString().length ?? 0) > 0) return;
    onSeek();
  };
  const wordLevel = Boolean(words?.length && onWordClick);
  return (
    <div
      className={clsx(
        "ds-msg",
        divided && "ds-msg--divided",
        active && "ds-msg--active",
      )}
    >
      <div className="ds-msg__head">
        <span
          className={clsx("ds-msg__speaker", isUser && "ds-msg__speaker--me")}
        >
          {speaker}
        </span>
        {onSeek ? (
          <button
            className="ds-msg__ts"
            onClick={onSeek}
            title="Jump to this moment"
          >
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
        {wordLevel
          ? words!.map((word, i) => (
              <span key={i}>
                {i > 0 && " "}
                <span
                  className={clsx(
                    "ds-msg__word",
                    i === activeWordIndex && "ds-msg__word--active",
                  )}
                  title="Jump to this word"
                  onClick={(e) => {
                    e.stopPropagation();
                    if ((window.getSelection()?.toString().length ?? 0) > 0)
                      return;
                    onWordClick!(word.start);
                  }}
                >
                  {word.text}
                </span>
              </span>
            ))
          : content}
        {partial && (
          <span className="ds-msg__interim">
            {content ? ` ${partial}` : partial}
          </span>
        )}
      </p>
      {translation && <p className="ds-msg__translation">{translation}</p>}
    </div>
  );
};
