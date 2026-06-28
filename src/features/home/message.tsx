import { Message as DSMessage } from "@/design-system";

interface MessageProps {
  label: string;
  timestamp: string;
  content: string;
  isUser?: boolean;
  onSeek?: () => void;
  active?: boolean;
}

/** Transcript line — thin wrapper over the design-system Message. */
export const Message = ({ label, timestamp, content, isUser, onSeek, active }: MessageProps) => (
  <DSMessage
    speaker={label}
    timestamp={timestamp}
    content={content}
    isUser={isUser}
    onSeek={onSeek}
    active={active}
  />
);
