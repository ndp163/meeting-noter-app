import { Play, Pause, SkipBack, SkipForward } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { useWavesurfer } from "@wavesurfer/react";
import { convertFileSrc } from "@tauri-apps/api/core";

interface AudioPlayerProps {
  audioPath?: string;
}

export const AudioPlayer = ({ audioPath }: AudioPlayerProps) => {
  const containerRef = useRef<HTMLDivElement>(null);
  const [currentTime, setCurrentTime] = useState("00:00");
  const [totalTime, setTotalTime] = useState("00:00");

  const { wavesurfer, isReady, isPlaying } = useWavesurfer({
    container: containerRef,
    url: audioPath ? convertFileSrc(audioPath) : undefined,
    waveColor: "#404a4c33",
    progressColor: "#2a4b8b",
    cursorColor: "#2a4b8b",
    barWidth: 2,
    barGap: 1,
    barRadius: 2,
    height: 87,
    normalize: true,
    hideScrollbar: true,
  });

  useEffect(() => {
    if (!wavesurfer) return;

    const subscriptions = [
      wavesurfer.on("ready", () => {
        const duration = wavesurfer.getDuration();
        setTotalTime(formatTime(duration));
      }),
      wavesurfer.on("timeupdate", (time) => {
        setCurrentTime(formatTime(time));
      }),
    ];

    return () => {
      subscriptions.forEach((unsub) => unsub());
    };
  }, [wavesurfer]);

  const handlePlayPause = () => {
    if (wavesurfer) {
      wavesurfer.playPause();
    }
  };

  const handleSkipBackward = () => {
    if (wavesurfer) {
      const currentTime = wavesurfer.getCurrentTime();
      wavesurfer.setTime(Math.max(0, currentTime - 10));
    }
  };

  const handleSkipForward = () => {
    if (wavesurfer) {
      const currentTime = wavesurfer.getCurrentTime();
      const duration = wavesurfer.getDuration();
      wavesurfer.setTime(Math.min(duration, currentTime + 10));
    }
  };

  return (
    <div className="flex gap-2.5 items-center w-full">
      {/* Play/Pause Controls */}
      <div className="flex gap-3 items-center justify-center p-[13px] w-[200px]">
        <button
          onClick={handleSkipBackward}
          disabled={!isReady || !audioPath}
          className="w-6 h-6 flex items-center justify-center hover:scale-110 transition-transform disabled:opacity-30 disabled:cursor-not-allowed"
        >
          <SkipBack className="w-full h-full text-custom-text-primary" />
        </button>

        <button
          onClick={handlePlayPause}
          disabled={!isReady || !audioPath}
          className="w-[30px] h-[30px] flex items-center justify-center hover:scale-110 transition-transform disabled:opacity-30 disabled:cursor-not-allowed"
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

        <button
          onClick={handleSkipForward}
          disabled={!isReady || !audioPath}
          className="w-6 h-6 flex items-center justify-center hover:scale-110 transition-transform disabled:opacity-30 disabled:cursor-not-allowed"
        >
          <SkipForward className="w-full h-full text-custom-text-primary" />
        </button>

        <p className="text-base text-custom-text-primary whitespace-nowrap ml-2">
          {currentTime} / {totalTime}
        </p>
      </div>

      {/* Waveform */}
      <div className="h-[87px] flex-1 bg-custom-bg-secondary rounded-lg overflow-hidden">
        {audioPath ? (
          <div ref={containerRef} className="w-full h-full" />
        ) : (
          <div className="w-full h-full flex items-center justify-center text-custom-text-secondary text-sm">
            No audio available
          </div>
        )}
      </div>
    </div>
  );
};

const formatTime = (seconds: number): string => {
  const mins = Math.floor(seconds / 60);
  const secs = Math.floor(seconds % 60);
  return `${mins.toString().padStart(2, "0")}:${secs
    .toString()
    .padStart(2, "0")}`;
};
