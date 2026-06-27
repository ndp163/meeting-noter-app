import { type ReactNode } from "react";
import { Check } from "lucide-react";
import { clsx } from "./util";

export type StepState = "todo" | "active" | "done";

export interface StepProps {
  /** 1-based index shown in the bubble (replaced by a tick when done). */
  index: number;
  /** Step title. */
  label: string;
  /** Progress state. @default "todo" */
  state?: StepState;
  /** Short trailing hint, e.g. "Granted" or "412 / 640 MB". */
  hint?: string;
  /** Extra content under the label (e.g. a ProgressBar). */
  children?: ReactNode;
}

/** Onboarding checklist row: numbered bubble, label, optional hint / inline body. */
export const Step = ({ index, label, state = "todo", hint, children }: StepProps) => (
  <div className={clsx("ds-step", state !== "todo" && `ds-step--${state}`)}>
    <span className="ds-step__num">
      {state === "done" ? <Check size={15} /> : index}
    </span>
    <div className="ds-step__body">
      <div className="ds-step__label">{label}</div>
      {children}
    </div>
    {hint && <span className="ds-step__hint">{hint}</span>}
  </div>
);
