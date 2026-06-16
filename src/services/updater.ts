import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";

// Ask GitHub Releases whether a newer signed bundle exists. Returns the pending
// Update (with version + notes) or null when already current. Errors (offline,
// etc.) resolve to null so a failed check never blocks the UI.
export const checkForUpdate = async (): Promise<Update | null> => {
  try {
    return await check();
  } catch (err) {
    console.error("Update check failed:", err);
    return null;
  }
};

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
