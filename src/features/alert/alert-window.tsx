import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getMeetingDetectionStatus } from "@/services/meeting-detector";

/**
 * Standalone UI rendered in the always-on-top "alert" overlay window. It floats
 * above the meeting app (Teams, Zoom, ...) so the user can start recording
 * without switching back to the main window.
 */
export const AlertWindow = () => {
  const [appName, setAppName] = useState<string>("A meeting app");

  useEffect(() => {
    // Make the window background show through (see main.tsx).
    let unlisten: (() => void) | null = null;

    // The name may arrive via the event emitted on window creation, or we
    // fetch it ourselves in case the listener wasn't attached in time.
    getMeetingDetectionStatus()
      .then((name) => name && setAppName(name))
      .catch(() => {});

    listen<string>("meeting-detected", (event) => {
      if (event.payload) setAppName(event.payload);
    }).then((release) => {
      unlisten = release;
    });

    return () => {
      unlisten?.();
    };
  }, []);

  return (
    <div className="flex h-screen w-screen items-center gap-3 rounded-[var(--ds-radius)] border border-[var(--ds-border)] bg-[var(--ds-surface)] px-4 py-2 shadow-[var(--ds-shadow-lg)]">
      <span className="h-2 w-2 shrink-0 rounded-full bg-[var(--ds-rec)] animate-pulse" />
      <span className="flex-1 truncate text-sm text-[var(--ds-text)]">
        {appName} is using the microphone
      </span>
      <button
        onClick={() => void invoke("start_recording_from_alert")}
        className="shrink-0 rounded-full bg-[var(--ds-rec)] px-3 py-1 text-sm text-[var(--ds-on-accent)] cursor-pointer"
      >
        Start recording
      </button>
      <button
        onClick={() => void invoke("dismiss_alert")}
        aria-label="Dismiss"
        className="shrink-0 text-[var(--ds-text-2)] cursor-pointer"
      >
        ✕
      </button>
    </div>
  );
};
