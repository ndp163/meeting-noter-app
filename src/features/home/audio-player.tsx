import { Play, Pause } from "lucide-react";

interface AudioPlayerProps {
  currentTime: string;
  totalTime: string;
  isPlaying: boolean;
  onPlayPause: () => void;
  waveformUrl?: string;
}

export const AudioPlayer = ({
  currentTime,
  totalTime,
  isPlaying,
  onPlayPause,
  waveformUrl = "https://www.figma.com/api/mcp/asset/7525f6bc-6684-4f23-abe0-ee48c6ea3fcb",
}: AudioPlayerProps) => {
  return (
    <div className="flex gap-2.5 items-center w-full">
      {/* Play/Pause Controls */}
      <div className="flex gap-5 items-center justify-center p-[13px] w-[173px]">
        <button
          onClick={onPlayPause}
          className="w-[30px] h-[30px] flex items-center justify-center hover:scale-110 transition-transform"
        >
          {isPlaying ? (
            <Pause
              className="w-full h-full text-custom-text-primary"
              fill="#404a4c"
            />
          ) : (
            <Play
              className="w-full h-full text-custom-text-primary"
              fill="#404a4c"
            />
          )}
        </button>
        <p className="text-base text-custom-text-primary whitespace-nowrap">
          {currentTime} / {totalTime}
        </p>
      </div>

      {/* Waveform */}
      <div className="h-[87px] flex-1">
        <img
          src={waveformUrl}
          alt="Audio waveform"
          className="w-full h-full object-cover"
        />
      </div>
    </div>
  );
};
