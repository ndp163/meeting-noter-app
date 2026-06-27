import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { MeetingLanguage } from "@/types/meeting";

const SETUP_PROGRESS_EVENT = "setup://progress";

/** Real byte-weighted download progress for one language, fraction 0.0–1.0. */
export interface SetupProgress {
  language: MeetingLanguage;
  fraction: number;
}

/** Whether at least one language is installed (skip onboarding if true). */
export const modelsReady = (): Promise<boolean> => invoke<boolean>("models_ready");

/** Languages currently installed (ASR + diarizer cached on disk). */
export const installedLanguages = (): Promise<MeetingLanguage[]> =>
  invoke<MeetingLanguage[]>("installed_languages");

/** Download one language's model; rejects on failure so the UI can retry. */
export const downloadLanguage = (language: MeetingLanguage): Promise<void> =>
  invoke("download_language", { language });

/** Remove a language's ASR model to reclaim disk (shared diarizer is kept). */
export const deleteLanguage = (language: MeetingLanguage): Promise<void> =>
  invoke("delete_language", { language });

/** Subscribe to per-language download progress. Returns an unlisten fn. */
export const onSetupProgress = (handler: (progress: SetupProgress) => void) =>
  listen<SetupProgress>(SETUP_PROGRESS_EVENT, (e) => handler(e.payload));
