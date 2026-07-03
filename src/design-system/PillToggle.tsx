import { useRef } from "react";
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
  /** Accessible name for the group. */
  "aria-label"?: string;
  className?: string;
}

/** Rounded pill segmented control — a single-select radio group. Arrow keys
 *  move between options (roving tabindex). */
export const PillToggle = ({
  options,
  value,
  onChange,
  className,
  "aria-label": ariaLabel,
}: PillToggleProps) => {
  const refs = useRef<(HTMLButtonElement | null)[]>([]);

  const move = (to: number) => {
    const i = (to + options.length) % options.length;
    onChange?.(options[i].id);
    refs.current[i]?.focus();
  };

  const onKeyDown = (e: React.KeyboardEvent, index: number) => {
    if (e.key === "ArrowRight" || e.key === "ArrowDown") {
      e.preventDefault();
      move(index + 1);
    } else if (e.key === "ArrowLeft" || e.key === "ArrowUp") {
      e.preventDefault();
      move(index - 1);
    }
  };

  return (
    <div
      className={clsx("ds-pillgroup", className)}
      role="radiogroup"
      aria-label={ariaLabel}
    >
      {options.map((o, i) => (
        <button
          key={o.id}
          ref={(el) => {
            refs.current[i] = el;
          }}
          role="radio"
          aria-checked={o.id === value}
          tabIndex={o.id === value ? 0 : -1}
          className={clsx("ds-pill", o.id === value && "ds-pill--active")}
          onClick={() => onChange?.(o.id)}
          onKeyDown={(e) => onKeyDown(e, i)}
        >
          {o.label}
        </button>
      ))}
    </div>
  );
};
