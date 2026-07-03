import { type ReactNode } from "react";
import { Modal } from "./Modal";
import { Button, type ButtonVariant } from "./Button";

export interface ConfirmDialogProps {
  open: boolean;
  title: string;
  /** Body text or nodes explaining the consequence. */
  message: ReactNode;
  /** Confirm button label. @default "Confirm" */
  confirmLabel?: string;
  /** Cancel button label. @default "Cancel" */
  cancelLabel?: string;
  /** Confirm button style — use "danger" for destructive actions. @default "primary" */
  confirmVariant?: ButtonVariant;
  /** Optional leading icon on the confirm button. */
  confirmIcon?: ReactNode;
  /** Disable confirm (e.g. an action is blocked). */
  confirmDisabled?: boolean;
  onConfirm: () => void;
  onCancel: () => void;
}

/**
 * A yes/no confirmation dialog on top of Modal. One place for every "are you
 * sure?" prompt so destructive actions look and behave consistently.
 */
export const ConfirmDialog = ({
  open,
  title,
  message,
  confirmLabel = "Confirm",
  cancelLabel = "Cancel",
  confirmVariant = "primary",
  confirmIcon,
  confirmDisabled,
  onConfirm,
  onCancel,
}: ConfirmDialogProps) => (
  <Modal open={open} title={title} onClose={onCancel}>
    <div className="flex flex-col gap-4">
      <div className="text-sm text-[var(--ds-text-2)]">{message}</div>
      <div className="flex justify-end gap-2">
        <Button variant="ghost" onClick={onCancel}>
          {cancelLabel}
        </Button>
        <Button
          variant={confirmVariant}
          icon={confirmIcon}
          disabled={confirmDisabled}
          onClick={onConfirm}
        >
          {confirmLabel}
        </Button>
      </div>
    </div>
  </Modal>
);
