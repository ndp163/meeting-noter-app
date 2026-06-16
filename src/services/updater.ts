import { check } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";

// Check GitHub Releases for a newer signed bundle. If found, download, install,
// and relaunch. No-op when already up to date. Errors are swallowed so a failed
// check (offline, etc.) never blocks app startup.
export const checkForUpdates = async (): Promise<void> => {
  try {
    const update = await check();
    if (!update) return;

    await update.downloadAndInstall();
    await relaunch();
  } catch (err) {
    console.error("Update check failed:", err);
  }
};
