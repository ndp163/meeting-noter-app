import { Play, Pause } from "lucide-react";
import { clsx } from "./util";

export interface AudioPlayerProps {
  /** Current position, formatted (e.g. "00:31"). */
  current?: string;
  /** Total duration, formatted (e.g. "02:14"). */
  duration?: string;
  /** Progress ratio 0–1 — colours the waveform up to this point. */
  progress?: number;
  /** Show the pause glyph instead of play. */
  playing?: boolean;
  onToggle?: () => void;
  className?: string;
}

/* Static bar heights — a representative waveform silhouette. */
const BARS = [
  30, 55, 80, 45, 65, 90, 40, 70, 50, 85, 35, 60, 75, 45, 55, 30, 68, 48,
  62, 78, 42, 58, 88, 38, 72, 52,
];

/**
 * Audio scrubber — visual design component.
 * Renders a static waveform silhouette; in the app a wavesurfer engine drives
 * playback. For design purposes this shows the control's look and progress state.
 */
export const AudioPlayer = ({
  current = "00:00",
  duration = "00:00",
  progress = 0,
  playing,
  onToggle,
  className,
}: AudioPlayerProps) => {
  const onCount = Math.round(Math.max(0, Math.min(1, progress)) * BARS.length);
  return (
    <div className={clsx("ds-player", className)}>
      <button className="ds-player__play" onClick={onToggle} aria-label={playing ? "Pause" : "Play"}>
        {playing ? <Pause size={16} fill="currentColor" /> : <Play size={16} fill="currentColor" />}
      </button>
      <div className="ds-player__wave">
        {BARS.map((h, i) => (
          <span key={i} className={clsx(i < onCount && "ds-on")} style={{ height: `${h}%` }} />
        ))}
      </div>
      <span className="ds-player__time">
        {current} / {duration}
      </span>
    </div>
  );
};
