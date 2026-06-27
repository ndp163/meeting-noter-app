import { clsx } from "./util";

export type StatusTone = "idle" | "rec" | "accent" | "ok";

export interface StatusDotProps {
  /** Colour of the dot. @default "idle" */
  tone?: StatusTone;
  /** Animate with a slow pulse (use for live states). */
  pulse?: boolean;
  /** Optional trailing label, rendered beside the dot. */
  label?: string;
  className?: string;
}

const TONE: Record<StatusTone, string> = {
  idle: "",
  rec: "ds-dot--rec",
  accent: "ds-dot--accent",
  ok: "ds-dot--ok",
};

/** Small status indicator dot, optionally pulsing, optionally with a label. */
export const StatusDot = ({ tone = "idle", pulse, label, className }: StatusDotProps) => {
  const dot = (
    <span className={clsx("ds-dot", TONE[tone], pulse && "ds-dot--pulse")} />
  );
  if (!label) return <span className={className}>{dot}</span>;
  return (
    <span className={clsx("ds-status", className)}>
      {dot}
      {label}
    </span>
  );
};
