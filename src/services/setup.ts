import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { MeetingLanguage } from "@/types/meeting";

const SETUP_PROGRESS_EVENT = "setup://progress";

/** Self-hosted model manifest. Mirror of `MODELS_MANIFEST_URL` in
 *  `src-tauri/src/config.rs` — keep both in sync when bumping the version. */
const MODELS_MANIFEST_URL =
  "https://meeting-noter.s3.ap-southeast-1.amazonaws.com/v1/manifest.json";

/** Real per-language download size (bytes), read from the manifest `sizes`
 *  summary so labels reflect the actual files on the server, not a guess.
 *  Returns null on network failure (UI falls back to its static estimate). */
export const fetchModelSizes = async (): Promise<Partial<
  Record<MeetingLanguage, number>
> | null> => {
  try {
    const res = await fetch(MODELS_MANIFEST_URL, { cache: "no-store" });
    if (!res.ok) return null;
    const manifest = (await res.json()) as {
      sizes?: Partial<Record<MeetingLanguage, number>>;
    };
    return manifest.sizes ?? null;
  } catch {
    return null;
  }
};

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
