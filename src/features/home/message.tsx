import { Message as DSMessage } from "@/design-system";

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
}

/** Transcript line — thin wrapper over the design-system Message. */
export const Message = ({ label, timestamp, content, partial, translation, isUser, onSeek, active, pending }: MessageProps) => (
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
  />
);
