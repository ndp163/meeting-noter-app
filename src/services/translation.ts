import { invoke } from "@tauri-apps/api/core";
import { listen, UnlistenFn } from "@tauri-apps/api/event";

const TRANSLATION_EVENT = "translation://chunk";

export interface TranslationEventPayload {
  source: "mic" | "speaker";
  text: string; // translated text
  start_sec: number; // matches the transcript message's audioOffset
  is_result_final: boolean;
  is_sentence_final: boolean;
}

/** `"installed"` (offline-ready) · `"supported"` (needs download) · `"unsupported"`. */
export type TranslateAvailability = "installed" | "supported" | "unsupported";

export type TranscriptView = "translated" | "both";

export interface TranslateConfig {
  enabled: boolean;
  target: string; // BCP-47 language code, "" = unset
  view: TranscriptView;
}

export const getTranslateConfig = async (): Promise<TranslateConfig> =>
  invoke<TranslateConfig>("get_translate_config");

export const setTranslateConfig = async (
  enabled: boolean,
  target: string,
  view: TranscriptView
): Promise<void> => {
  await invoke("set_translate_config", { enabled, target, view });
};

export const translateStatus = async (
  source: string,
  target: string
): Promise<TranslateAvailability> =>
  invoke<TranslateAvailability>("translate_status", { source, target });

/** Trigger the one-time pack-download consent sheet; resolves when done. */
export const translateDownload = async (
  source: string,
  target: string
): Promise<void> => {
  await invoke("translate_download", { source, target });
};

export const listenToTranslation = async (
  handler: (payload: TranslationEventPayload) => void
): Promise<UnlistenFn> =>
  listen<TranslationEventPayload>(TRANSLATION_EVENT, (event) =>
    handler(event.payload)
  );
