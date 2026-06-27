import { clsx } from "./util";

export interface ProgressBarProps {
  /** Completion ratio, 0–1. Ignored when `indeterminate`. */
  value?: number;
  /** Show a looping slide animation when the percentage is unknown. */
  indeterminate?: boolean;
  className?: string;
}

/** Thin gradient progress bar (e.g. model download). */
export const ProgressBar = ({ value = 0, indeterminate, className }: ProgressBarProps) => {
  const pct = Math.max(0, Math.min(1, value)) * 100;
  return (
    <div
      className={clsx("ds-progress", indeterminate && "ds-progress--indeterminate", className)}
      role="progressbar"
      aria-valuenow={indeterminate ? undefined : Math.round(pct)}
    >
      <span className="ds-progress__fill" style={indeterminate ? undefined : { width: `${pct}%` }} />
    </div>
  );
};
