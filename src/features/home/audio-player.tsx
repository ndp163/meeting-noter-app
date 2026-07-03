import { Play, Pause } from "lucide-react";
import {
  forwardRef,
  useEffect,
  useImperativeHandle,
  useRef,
  useState,
} from "react";
import { useWavesurfer } from "@wavesurfer/react";
import { convertFileSrc } from "@tauri-apps/api/core";

interface AudioPlayerProps {
  audioPath?: string;
  /** Fires on every playback tick with the absolute position (seconds). */
  onTimeUpdate?: (seconds: number) => void;
}

export interface AudioPlayerHandle {
  /** Seek to an absolute position (seconds) and start playing. */
  seek: (seconds: number) => void;
}

export const AudioPlayer = forwardRef<AudioPlayerHandle, AudioPlayerProps>(
  ({ audioPath, onTimeUpdate }, ref) => {
  const containerRef = useRef<HTMLDivElement>(null);
  const [currentTime, setCurrentTime] = useState("00:00");
  const [totalTime, setTotalTime] = useState("00:00");
  const [loadError, setLoadError] = useState(false);
  // Keep the latest callback in a ref so the timeupdate subscription stays stable.
  const onTimeUpdateRef = useRef(onTimeUpdate);
  onTimeUpdateRef.current = onTimeUpdate;

  const { wavesurfer, isReady, isPlaying } = useWavesurfer({
    container: containerRef,
    url: audioPath ? convertFileSrc(audioPath) : undefined,
    waveColor: "#d7c8b0",
    progressColor: "#b5552f",
    cursorColor: "#b5552f",
    barWidth: 2,
    barGap: 1,
    barRadius: 2,
    height: 56,
    normalize: true,
    hideScrollbar: true,
  });

  useEffect(() => {
    setLoadError(false);
    if (!audioPath) {
      setCurrentTime("00:00");
      setTotalTime("00:00");
    }
  }, [audioPath]);

  useEffect(() => {
    if (!wavesurfer) {
      setCurrentTime("00:00");
      setTotalTime("00:00");
      return;
    }

    const subscriptions = [
      wavesurfer.on("ready", () => {
        const duration = wavesurfer.getDuration();
        setTotalTime(formatTime(duration));
      }),
      wavesurfer.on("timeupdate", (time) => {
        setCurrentTime(formatTime(time));
        onTimeUpdateRef.current?.(time);
      }),
      wavesurfer.on("error", (err) => {
        console.error("Audio load failed", err);
        setLoadError(true);
      }),
    ];

    return () => {
      subscriptions.forEach((unsub) => unsub());
    };
  }, [wavesurfer]);

  useImperativeHandle(
    ref,
    () => ({
      seek: (seconds: number) => {
        if (!wavesurfer) return;
        const duration = wavesurfer.getDuration();
        if (duration <= 0) return;
        wavesurfer.setTime(Math.max(0, Math.min(seconds, duration)));
        void wavesurfer.play();
      },
    }),
    [wavesurfer],
  );

  const handlePlayPause = () => {
    if (wavesurfer) {
      wavesurfer.playPause();
    }
  };

  return (
    <div className="ds-player">
      <button
        onClick={handlePlayPause}
        disabled={!isReady || !audioPath}
        aria-label={isPlaying ? "Pause" : "Play"}
        className="ds-player__play disabled:opacity-30 disabled:cursor-not-allowed"
      >
        {isPlaying ? (
          <Pause size={16} fill="currentColor" />
        ) : (
          <Play size={16} fill="currentColor" />
        )}
      </button>

      {/* Waveform */}
      <div className="h-[56px] flex-1 overflow-hidden">
        {!audioPath ? (
          <div className="w-full h-full flex items-center justify-center text-[var(--ds-text-2)] text-sm">
            No audio available
          </div>
        ) : loadError ? (
          <div className="w-full h-full flex items-center justify-center text-[var(--ds-rec)] text-sm">
            Couldn&apos;t load the audio
          </div>
        ) : (
          <div ref={containerRef} className="w-full h-full" />
        )}
      </div>

      <span className="ds-player__time">
        {currentTime} / {totalTime}
      </span>
    </div>
  );
  },
);

AudioPlayer.displayName = "AudioPlayer";

const formatTime = (seconds: number): string => {
  const mins = Math.floor(seconds / 60);
  const secs = Math.floor(seconds % 60);
  return `${mins.toString().padStart(2, "0")}:${secs
    .toString()
    .padStart(2, "0")}`;
};
