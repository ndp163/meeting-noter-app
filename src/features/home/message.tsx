import { Message as DSMessage, type MessageWord } from "@/design-system";

interface MessageProps {
  label: string;
  timestamp: string;
  content: string;
  partial?: string;
  translation?: string;
  isUser?: boolean;
  onSeek?: () => void;
  active?: boolean;
  pending?: boolean;
  words?: MessageWord[];
  activeWordIndex?: number;
  onWordClick?: (seconds: number) => void;
}

/** Transcript line — thin wrapper over the design-system Message. */
export const Message = ({
  label,
  timestamp,
  content,
  partial,
  translation,
  isUser,
  onSeek,
  active,
  pending,
  words,
  activeWordIndex,
  onWordClick,
}: MessageProps) => (
  <DSMessage
    speaker={label}
    timestamp={timestamp}
    content={content}
    partial={partial}
    translation={translation}
    isUser={isUser}
    onSeek={onSeek}
    active={active}
    pending={pending}
    words={words}
    activeWordIndex={activeWordIndex}
    onWordClick={onWordClick}
  />
);
