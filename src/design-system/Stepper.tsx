import { useState, type KeyboardEvent } from "react";
import { Minus, Plus } from "lucide-react";
import { clsx } from "./util";

export interface StepperProps {
  /** Current count, or `null` for the unset/automatic state. */
  value: number | null;
  /** Fired with the new count, or `null` when stepping back to automatic. */
  onChange?: (value: number | null) => void;
  /** Lowest pickable number; stepping below it returns to `null`. */
  min?: number;
  /** Highest pickable number. */
  max?: number;
  /** Text shown in the unset state (e.g. "Auto"). */
  placeholder?: string;
  /** Accessible name for the value field. */
  "aria-label"?: string;
  disabled?: boolean;
  className?: string;
}

/**
 * Compact numeric stepper: `[−] value [+]` in one bordered group. The value is
 * also directly editable — typing commits on blur/Enter, clamped to
 * `min…max`; clearing it (or stepping below `min`) returns to the `null`
 * "automatic" state, shown as `placeholder`. Arrow keys step too.
 * Presentational — the caller owns `value`.
 */
export const Stepper = ({
  value,
  onChange,
  min = 1,
  max = 99,
  placeholder = "Auto",
  disabled,
  className,
  "aria-label": ariaLabel,
}: StepperProps) => {
  // Free-typed text while the field is focused; null = not editing.
  const [draft, setDraft] = useState<string | null>(null);

  const clamp = (n: number) => Math.min(max, Math.max(min, n));

  const commitDraft = () => {
    if (draft === null) return;
    const parsed = Number.parseInt(draft, 10);
    onChange?.(Number.isFinite(parsed) ? clamp(parsed) : null);
    setDraft(null);
  };

  const step = (delta: number) => {
    setDraft(null);
    if (value === null) {
      if (delta > 0) onChange?.(min);
    } else {
      const next = value + delta;
      onChange?.(next < min ? null : clamp(next));
    }
  };

  const onKeyDown = (e: KeyboardEvent<HTMLInputElement>) => {
    if (e.key === "Enter") {
      commitDraft();
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      step(1);
    } else if (e.key === "ArrowDown") {
      e.preventDefault();
      step(-1);
    }
  };

  const shown = draft ?? (value === null ? "" : String(value));

  return (
    <div
      className={clsx(
        "ds-stepper",
        disabled && "ds-stepper--disabled",
        className,
      )}
    >
      <button
        type="button"
        className="ds-stepper__btn"
        aria-label="Decrease"
        disabled={disabled || value === null}
        onClick={() => step(-1)}
        tabIndex={-1}
      >
        <Minus />
      </button>
      <input
        className="ds-stepper__input"
        type="text"
        inputMode="numeric"
        role="spinbutton"
        aria-label={ariaLabel}
        aria-valuemin={min}
        aria-valuemax={max}
        aria-valuenow={value ?? undefined}
        aria-valuetext={value === null ? placeholder : String(value)}
        placeholder={placeholder}
        value={shown}
        disabled={disabled}
        onChange={(e) => setDraft(e.target.value.replace(/[^\d]/g, ""))}
        onBlur={commitDraft}
        onKeyDown={onKeyDown}
      />
      <button
        type="button"
        className="ds-stepper__btn"
        aria-label="Increase"
        disabled={disabled || value === max}
        onClick={() => step(1)}
        tabIndex={-1}
      >
        <Plus />
      </button>
    </div>
  );
};
