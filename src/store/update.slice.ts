import { StateCreator } from "zustand";
import { immer } from "zustand/middleware/immer";
import type { Update } from "@tauri-apps/plugin-updater";
import {
  checkForUpdate,
  installUpdate,
  getCurrentVersion,
} from "@/services/updater";

export type UpdatePhase =
  | "idle" // no check run yet
  | "checking" // check in flight
  | "available" // newer version found
  | "uptodate" // checked, already current
  | "installing" // download + install in flight
  | "error"; // check failed (offline etc.)

/** Serializable mirror of the live Update, safe to hold in immer state. */
export interface UpdateInfo {
  version: string;
  currentVersion: string;
  date?: string;
  notes?: string;
}

export interface UpdateSlice {
  updatePhase: UpdatePhase;
  updateInfo: UpdateInfo | null;
  /** Download progress, 0–1, while installing. */
  updateProgress: number;
  /** Running app version, `null` until loaded. */
  currentVersion: string | null;
  /** Banner hidden by the user for this session. */
  updateDismissed: boolean;

  loadCurrentVersion: () => Promise<void>;
  checkUpdate: () => Promise<void>;
  runInstall: () => Promise<void>;
  dismissUpdate: () => void;
}

// The live Update is kept OUT of immer state: its methods rely on an internal
// resource handle that immer's deep-freeze would lock. The slice mirrors only
// the data it needs for rendering.
let pendingUpdate: Update | null = null;

export const createUpdateSlice: StateCreator<UpdateSlice, [], [], UpdateSlice> =
  immer((set, get) => ({
    updatePhase: "idle",
    updateInfo: null,
    updateProgress: 0,
    currentVersion: null,
    updateDismissed: false,

    loadCurrentVersion: async () => {
      const v = await getCurrentVersion();
      set((s) => {
        s.currentVersion = v;
      });
    },

    checkUpdate: async () => {
      const phase = get().updatePhase;
      if (phase === "checking" || phase === "installing") return;
      set((s) => {
        s.updatePhase = "checking";
      });
      try {
        const update = await checkForUpdate();
        pendingUpdate = update;
        set((s) => {
          if (update) {
            s.updateInfo = {
              version: update.version,
              currentVersion: update.currentVersion,
              date: update.date,
              notes: update.body,
            };
            s.updatePhase = "available";
            s.updateDismissed = false;
          } else {
            s.updateInfo = null;
            s.updatePhase = "uptodate";
          }
        });
      } catch (err) {
        console.error("Update check failed:", err);
        set((s) => {
          s.updatePhase = "error";
        });
      }
    },

    runInstall: async () => {
      if (!pendingUpdate) return;
      set((s) => {
        s.updatePhase = "installing";
        s.updateProgress = 0;
      });
      try {
        await installUpdate(pendingUpdate, (fraction) =>
          set((s) => {
            s.updateProgress = fraction;
          }),
        );
        // installUpdate relaunches into the new version — nothing after runs.
      } catch (err) {
        console.error("Update install failed:", err);
        set((s) => {
          s.updatePhase = "available";
        });
      }
    },

    dismissUpdate: () =>
      set((s) => {
        s.updateDismissed = true;
      }),
  })) as StateCreator<UpdateSlice, [], [], UpdateSlice>;
