import { type ReactNode, useEffect, useId, useRef } from "react";
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
 * Centered dialog built on the native <dialog> element via `showModal()`, which
 * gives a focus trap, Escape handling, focus restore, and a top-layer backdrop
 * for free. Closes on Escape or backdrop click. Presentational — the caller
 * owns `open`. Styling stays on the `--ds-*` tokens (see components.css).
 */
export const Modal = ({ open, title, onClose, children, className }: ModalProps) => {
  const ref = useRef<HTMLDialogElement>(null);
  const titleId = useId();

  // Drive the native modal state from the `open` prop.
  useEffect(() => {
    const dlg = ref.current;
    if (!dlg) return;
    if (open && !dlg.open) dlg.showModal();
    else if (!open && dlg.open) dlg.close();
  }, [open]);

  // Lock background scroll while open (showModal makes it inert but still scrolls).
  useEffect(() => {
    if (!open) return;
    const prev = document.body.style.overflow;
    document.body.style.overflow = "hidden";
    return () => {
      document.body.style.overflow = prev;
    };
  }, [open]);

  return (
    <dialog
      ref={ref}
      className={clsx("ds-modal", className)}
      aria-labelledby={typeof title === "string" ? titleId : undefined}
      // Escape fires `cancel`; route it (and the resulting close) through
      // onClose. Guard on target: React delegates the non-bubbling
      // close/cancel events at the root, so a NESTED dialog's own events would
      // otherwise trigger this parent's handler and close it too.
      onCancel={(e) => {
        if (e.target !== ref.current) return;
        e.preventDefault();
        onClose();
      }}
      onClose={(e) => {
        if (e.target !== ref.current) return;
        onClose();
      }}
      // Clicks on the dialog element itself (the backdrop area) dismiss.
      onClick={(e) => {
        if (e.target === ref.current) onClose();
      }}
    >
      {open && (
        <div className="ds-modal__inner" onClick={(e) => e.stopPropagation()}>
          <div className="ds-modal__header">
            {typeof title === "string" ? (
              <span id={titleId} className="ds-modal__title">
                {title}
              </span>
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
      )}
    </dialog>
  );
};
