import type { MeetingLanguage } from "@/types/meeting";

/** Display + download metadata for each transcription language. */
export interface LanguageInfo {
  id: MeetingLanguage;
  /** English label, e.g. for settings rows. */
  label: string;
  /** Native label shown in the picker (e.g. 日本語). */
  native: string;
  /** Approximate on-disk download size (ASR model). Diarizer (~13 MB) is
   *  shared and downloaded once with the first language. */
  approxSize: string;
}

/** All supported languages, in display order. Keep in sync with the Rust
 *  `MeetingLanguage` enum and the ASR model versions (en → v2, ja → tdtJa,
 *  vi → standalone Parakeet-CTC). */
export const LANGUAGES: LanguageInfo[] = [
  { id: "en", label: "English", native: "English", approxSize: "Estimating" },
  { id: "ja", label: "Japanese", native: "日本語", approxSize: "Estimating" },
  { id: "vi", label: "Vietnamese", native: "Tiếng Việt", approxSize: "Estimating" },
];

export const languageInfo = (id: MeetingLanguage): LanguageInfo =>
  LANGUAGES.find((l) => l.id === id) ?? LANGUAGES[0];
