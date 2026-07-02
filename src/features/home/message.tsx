import { Message as DSMessage } from "@/design-system";

interface MessageProps {
  label: string;
  timestamp: string;
  content: string;
  translation?: string;
  isUser?: boolean;
  onSeek?: () => void;
  active?: boolean;
}

/** Transcript line — thin wrapper over the design-system Message. */
export const Message = ({ label, timestamp, content, translation, isUser, onSeek, active }: MessageProps) => (
  <DSMessage
    speaker={label}
    timestamp={timestamp}
    content={content}
    translation={translation}
    isUser={isUser}
    onSeek={onSeek}
    active={active}
  />
);
