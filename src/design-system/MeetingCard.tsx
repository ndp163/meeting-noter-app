import { clsx } from "./util";
import { StatusDot } from "./StatusDot";

export type MeetingSource = "Teams" | "Zoom" | "Meet";

export interface MeetingCardProps {
  /** Meeting title. Truncates on overflow. */
  title: string;
  /** Pre-formatted timestamp, e.g. "10:24 · 27/06/26". */
  timestamp: string;
  /** Detected meeting source — renders a small tag. */
  source?: MeetingSource;
  /** Currently recording — shows the red "Rec" pulse. */
  recording?: boolean;
  /** Highlighted/selected state. */
  active?: boolean;
  onClick?: () => void;
}

/** Sidebar list item for a single meeting. */
export const MeetingCard = ({
  title,
  timestamp,
  source,
  recording,
  active,
  onClick,
}: MeetingCardProps) => (
  <div
    className={clsx("ds-meeting", active && "ds-meeting--active")}
    onClick={onClick}
    role="button"
  >
    <div className="ds-meeting__row">
      <span className="ds-meeting__title">{title}</span>
      {recording && (
        <span style={{ display: "inline-flex", alignItems: "center", gap: 5, fontSize: 11, fontWeight: 600, color: "var(--ds-rec)" }}>
          <StatusDot tone="rec" pulse /> Rec
        </span>
      )}
    </div>
    <div className="ds-meeting__row">
      <span className="ds-meeting__meta">{timestamp}</span>
      {source && <span className="ds-src">{source}</span>}
    </div>
  </div>
);
