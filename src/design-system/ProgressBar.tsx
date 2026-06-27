import { clsx } from "./util";

export interface ProgressBarProps {
  /** Completion ratio, 0–1. */
  value: number;
  className?: string;
}

/** Thin gradient progress bar (e.g. model download). */
export const ProgressBar = ({ value, className }: ProgressBarProps) => {
  const pct = Math.max(0, Math.min(1, value)) * 100;
  return (
    <div className={clsx("ds-progress", className)} role="progressbar" aria-valuenow={Math.round(pct)}>
      <span className="ds-progress__fill" style={{ width: `${pct}%` }} />
    </div>
  );
};
