import { type ButtonHTMLAttributes } from "react";
import { Mic, Square } from "lucide-react";
import { clsx } from "./util";

export interface CaptureButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  /** Whether a recording is currently in progress. */
  capturing?: boolean;
}

/** Start / Stop capture control. Soft-red when idle, solid-red while recording. */
export const CaptureButton = ({ capturing, className, ...rest }: CaptureButtonProps) => (
  <button
    className={clsx("ds-capture", capturing && "ds-capture--active", className)}
    {...rest}
  >
    {capturing ? <Square size={18} /> : <Mic size={18} />}
    {capturing ? "Stop Capture" : "Start Capture"}
  </button>
);
