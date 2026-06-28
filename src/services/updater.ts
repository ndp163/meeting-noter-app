import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { getVersion } from "@tauri-apps/api/app";

// The version of the running app (from tauri.conf.json).
export const getCurrentVersion = (): Promise<string> => getVersion();

// Ask GitHub Releases whether a newer signed bundle exists. Returns the pending
// Update (with version + notes) or null when already current. Throws on failure
// (offline, network) — the caller decides whether to surface it (manual check)
// or swallow it (passive banner check).
export const checkForUpdate = (): Promise<Update | null> => check();

// Download + install the given update, reporting download progress (0..1) via
// onProgress, then relaunch into the new version. Caller drives this from a
// user action — never automatically.
export const installUpdate = async (
  update: Update,
  onProgress: (fraction: number) => void,
): Promise<void> => {
  let total = 0;
  let downloaded = 0;

  await update.downloadAndInstall((event) => {
    switch (event.event) {
      case "Started":
        total = event.data.contentLength ?? 0;
        break;
      case "Progress":
        downloaded += event.data.chunkLength;
        if (total > 0) onProgress(downloaded / total);
        break;
      case "Finished":
        onProgress(1);
        break;
    }
  });

  await relaunch();
};
