import { useEffect, useState } from "react";
import { Download, X } from "lucide-react";
import { listen } from "@tauri-apps/api/event";
import { useBoundStore } from "@/store";
import { ConfirmDialog } from "@/design-system";

/**
 * Passive update surface. Checks once on mount and slides a banner in at the top
 * of the window when a newer version exists; the user installs (download
 * progress, then relaunch) or dismisses for the session. Also listens for the
 * tray "Check for Updates…" item, which opens the Settings → Updates tab.
 *
 * All state lives in the update slice so the banner, the Settings tab, and the
 * tray share one source of truth and never double-check.
 */
export const UpdateBanner = () => {
  const phase = useBoundStore.use.updatePhase();
  const info = useBoundStore.use.updateInfo();
  const progress = useBoundStore.use.updateProgress();
  const dismissed = useBoundStore.use.updateDismissed();
  const checkUpdate = useBoundStore.use.checkUpdate();
  const runInstall = useBoundStore.use.runInstall();
  const dismissUpdate = useBoundStore.use.dismissUpdate();
  const loadCurrentVersion = useBoundStore.use.loadCurrentVersion();
  const openSettings = useBoundStore.use.openSettings();
  const isRecording = useBoundStore.use.isRecording();
  const [confirmInstall, setConfirmInstall] = useState(false);

  useEffect(() => {
    void loadCurrentVersion();
    void checkUpdate();

    const unlisten = listen("tray://check-update", () => {
      openSettings("updates");
      void checkUpdate();
    });
    return () => {
      void unlisten.then((off) => off());
    };
  }, [loadCurrentVersion, checkUpdate, openSettings]);

  const installing = phase === "installing";
  const showAvailable = phase === "available" && !dismissed && !!info;

  if (!installing && !showAvailable) return null;

  return (
    <>
    <div className="fixed top-0 inset-x-0 z-50 flex items-center gap-3 px-5 py-2.5 bg-[var(--ds-accent)] text-[var(--ds-on-accent)] text-sm shadow-[var(--ds-shadow)]">
      <Download className="w-4 h-4 shrink-0" />

      {installing ? (
        <>
          <span className="flex-1">
            Downloading update… {Math.round(progress * 100)}%
          </span>
          <div className="w-32 h-1.5 rounded-full bg-[var(--ds-on-accent)]/30 overflow-hidden">
            <div
              className="h-full bg-[var(--ds-on-accent)] transition-all"
              style={{ width: `${progress * 100}%` }}
            />
          </div>
        </>
      ) : (
        <>
          <span className="flex-1">Version {info!.version} available.</span>
          <button
            onClick={() => openSettings("updates")}
            className="underline underline-offset-2 opacity-90 hover:opacity-100"
          >
            What&apos;s new
          </button>
          <button
            onClick={() => setConfirmInstall(true)}
            className="font-semibold underline underline-offset-2"
          >
            Install &amp; restart
          </button>
          <button
            onClick={dismissUpdate}
            aria-label="Dismiss"
            className="shrink-0 opacity-70 hover:opacity-100"
          >
            <X className="w-4 h-4" />
          </button>
        </>
      )}
    </div>

    <ConfirmDialog
      open={confirmInstall}
      title="Install update & restart?"
      message={
        isRecording
          ? "A recording is in progress. Stop it before installing — restarting now would lose the in-progress recording."
          : "The app will download the update and restart. Any unsaved work will be interrupted."
      }
      confirmLabel="Install & restart"
      confirmDisabled={isRecording}
      confirmIcon={<Download className="w-4 h-4" />}
      onConfirm={() => {
        setConfirmInstall(false);
        void runInstall();
      }}
      onCancel={() => setConfirmInstall(false)}
    />
    </>
  );
};
