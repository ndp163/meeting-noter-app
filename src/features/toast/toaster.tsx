import { useEffect } from "react";
import { AlertCircle, CheckCircle2, Info, X } from "lucide-react";
import { useBoundStore } from "@/store";
import type { Toast, ToastTone } from "@/store/toast.slice";

const TONE: Record<ToastTone, { color: string; Icon: typeof Info }> = {
  error: { color: "var(--ds-rec)", Icon: AlertCircle },
  success: { color: "var(--ds-ok)", Icon: CheckCircle2 },
  info: { color: "var(--ds-text-2)", Icon: Info },
};

const AUTO_DISMISS_MS = 5000;

/** App-wide toast stack (bottom-right). Reads the toast slice; renders nothing
 *  when empty. Each toast auto-dismisses; errors persist until dismissed. */
export const Toaster = () => {
  const toasts = useBoundStore.use.toasts();
  if (toasts.length === 0) return null;
  return (
    <div className="fixed bottom-4 right-4 z-[60] flex flex-col gap-2 max-w-[360px]">
      {toasts.map((t) => (
        <ToastItem key={t.id} toast={t} />
      ))}
    </div>
  );
};

const ToastItem = ({ toast }: { toast: Toast }) => {
  const dismiss = useBoundStore.use.dismissToast();
  const { color, Icon } = TONE[toast.tone];

  // Auto-dismiss non-error toasts; errors stay until the user closes them.
  useEffect(() => {
    if (toast.tone === "error") return;
    const id = setTimeout(() => dismiss(toast.id), AUTO_DISMISS_MS);
    return () => clearTimeout(id);
  }, [toast.id, toast.tone, dismiss]);

  return (
    <div
      role={toast.tone === "error" ? "alert" : "status"}
      className="flex items-start gap-2.5 p-3 rounded-[var(--ds-radius-sm)] border shadow-[var(--ds-shadow-lg)] bg-[var(--ds-surface)] border-[var(--ds-border)]"
    >
      <Icon className="w-4 h-4 shrink-0 mt-0.5" style={{ color }} />
      <span className="text-sm text-[var(--ds-text)] flex-1">{toast.message}</span>
      <button
        onClick={() => dismiss(toast.id)}
        aria-label="Dismiss"
        className="shrink-0 text-[var(--ds-text-3)] hover:text-[var(--ds-text)]"
      >
        <X className="w-4 h-4" />
      </button>
    </div>
  );
};
