import { cn } from "@/lib/utils";

interface MessageProps {
  speaker: string;
  timestamp: string;
  content: string;
  isUser?: boolean;
}

export const Message = ({
  speaker,
  timestamp,
  content,
  isUser,
}: MessageProps) => {
  return (
    <div className="bg-white flex flex-col gap-2.5 w-full">
      {/* Header with speaker and timestamp */}
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
            {speaker}
          </p>
        </div>
        <p className="text-base text-custom-text-secondary">{timestamp}</p>
      </div>

      {/* Message content */}
      <p className="text-base text-custom-text-primary whitespace-pre-wrap">
        {content}
      </p>
    </div>
  );
};
