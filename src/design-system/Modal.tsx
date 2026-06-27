import { type ReactNode, useEffect } from "react";
import { X } from "lucide-react";
import { clsx } from "./util";
import { IconButton } from "./IconButton";

export interface ModalProps {
  open: boolean;
  title?: ReactNode;
  onClose: () => void;
  children?: ReactNode;
  className?: string;
}

/**
 * Centered overlay dialog on `--ds-*` tokens. Closes on Escape or scrim click;
 * clicks inside the surface don't bubble out. Presentational only — the caller
 * owns `open` state.
 */
export const Modal = ({ open, title, onClose, children, className }: ModalProps) => {
  useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent) => e.key === "Escape" && onClose();
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [open, onClose]);

  if (!open) return null;

  return (
    <div className="ds-modal__scrim" onClick={onClose} role="presentation">
      <div
        className={clsx("ds-modal", className)}
        onClick={(e) => e.stopPropagation()}
        role="dialog"
        aria-modal="true"
      >
        <div className="ds-modal__header">
          {typeof title === "string" ? (
            <span className="ds-modal__title">{title}</span>
          ) : (
            title
          )}
          <IconButton
            icon={<X className="w-5 h-5" />}
            label="Close"
            onClick={onClose}
          />
        </div>
        <div className="ds-modal__body">{children}</div>
      </div>
    </div>
  );
};
