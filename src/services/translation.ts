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

/** Target languages surfaced for translation (transcript + summary). Source of
 *  a translation is always the meeting's ASR language (en / ja). */
export const TRANSLATE_TARGETS: { id: string; label: string }[] = [
  { id: "vi", label: "Vietnamese" },
  { id: "en", label: "English" },
  { id: "ja", label: "Japanese" },
  { id: "zh", label: "Chinese" },
  { id: "ko", label: "Korean" },
  { id: "es", label: "Spanish" },
  { id: "fr", label: "French" },
];

/** Human label for a target code, or the code itself if unknown. */
export const targetLabel = (code: string): string =>
  TRANSLATE_TARGETS.find((t) => t.id === code)?.label ?? code;

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
