import { clsx } from "./util";

export interface PillOption {
  /** Stable id returned by onChange. */
  id: string;
  /** Visible label. */
  label: string;
}

export interface PillToggleProps {
  /** Options, left to right. */
  options: PillOption[];
  /** Id of the active option. */
  value: string;
  /** Fired with the id of the clicked option. */
  onChange?: (id: string) => void;
  className?: string;
}

/** Rounded pill segmented control (e.g. language toggle: Original / Tiếng Việt). */
export const PillToggle = ({ options, value, onChange, className }: PillToggleProps) => (
  <div className={clsx("ds-pillgroup", className)}>
    {options.map((o) => (
      <button
        key={o.id}
        className={clsx("ds-pill", o.id === value && "ds-pill--active")}
        onClick={() => onChange?.(o.id)}
      >
        {o.label}
      </button>
    ))}
  </div>
);
