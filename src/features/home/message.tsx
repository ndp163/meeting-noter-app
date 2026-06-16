import { cn } from "@/lib/utils";

interface MessageProps {
  label: string;
  timestamp: string;
  content: string;
  isUser?: boolean;
  onSeek?: () => void;
}

export const Message = ({
  label,
  timestamp,
  content,
  isUser,
  onSeek,
}: MessageProps) => {
  return (
    <div className="bg-white flex flex-col gap-2.5 w-full">
      {/* Header with label and timestamp */}
      <div className="flex gap-2.5 items-center">
        <div
          className={cn(
            "flex items-center justify-center border-b border-custom-primary",
            "pb-0.5"
          )}
        >
          <p
            className={cn(
              "text-base",
              isUser ? "text-custom-text-highlight" : "text-custom-text-primary"
            )}
          >
            {label}
          </p>
        </div>
        {onSeek ? (
          <button
            onClick={onSeek}
            title="Jump to this moment"
            className="text-base text-custom-text-secondary tabular-nums hover:text-custom-primary hover:underline"
          >
            {timestamp}
          </button>
        ) : (
          <p className="text-base text-custom-text-secondary">{timestamp}</p>
        )}
      </div>

      {/* Message content: clickable to seek when an audio offset is known */}
      <p
        onClick={onSeek}
        className={cn(
          "text-base text-custom-text-primary whitespace-pre-wrap",
          onSeek && "cursor-pointer hover:text-custom-primary"
        )}
      >
        {content}
      </p>
    </div>
  );
};
