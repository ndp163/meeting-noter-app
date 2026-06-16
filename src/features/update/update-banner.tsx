import { useEffect, useState } from "react";
import { Download, X } from "lucide-react";
import type { Update } from "@tauri-apps/plugin-updater";
import { checkForUpdate, installUpdate } from "@/services/updater";

type Phase = "idle" | "available" | "installing";

/**
 * Checks for an update once on mount. When one exists it slides in a banner at
 * the top of the window asking the user to install. Installing shows download
 * progress, then relaunches into the new version. Dismissing hides the banner
 * until the next launch.
 */
export const UpdateBanner = () => {
  const [phase, setPhase] = useState<Phase>("idle");
  const [update, setUpdate] = useState<Update | null>(null);
  const [progress, setProgress] = useState(0);

  useEffect(() => {
    checkForUpdate().then((u) => {
      if (u) {
        setUpdate(u);
        setPhase("available");
      }
    });
  }, []);

  if (phase === "idle" || !update) return null;

  const handleInstall = async () => {
    setPhase("installing");
    try {
      await installUpdate(update, setProgress);
    } catch (err) {
      console.error("Update install failed:", err);
      setPhase("available");
    }
  };

  return (
    <div className="fixed top-0 inset-x-0 z-50 flex items-center gap-3 px-5 py-2.5 bg-custom-text-highlight text-white text-sm shadow-md">
      <Download className="w-4 h-4 shrink-0" />

      {phase === "available" ? (
        <>
          <span className="flex-1">
            Version {update.version} available.
          </span>
          <button
            onClick={handleInstall}
            className="font-semibold underline underline-offset-2"
          >
            Install &amp; restart
          </button>
          <button
            onClick={() => setPhase("idle")}
            aria-label="Dismiss"
            className="shrink-0 opacity-70 hover:opacity-100"
          >
            <X className="w-4 h-4" />
          </button>
        </>
      ) : (
        <>
          <span className="flex-1">
            Downloading update… {Math.round(progress * 100)}%
          </span>
          <div className="w-32 h-1.5 rounded-full bg-white/30 overflow-hidden">
            <div
              className="h-full bg-white transition-all"
              style={{ width: `${progress * 100}%` }}
            />
          </div>
        </>
      )}
    </div>
  );
};
