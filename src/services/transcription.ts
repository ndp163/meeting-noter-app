import { invoke } from "@tauri-apps/api/core";
import { listen, UnlistenFn } from "@tauri-apps/api/event";

const TRANSCRIPTION_EVENT = "transcription://chunk";

interface RecorderStatusResponse {
  active: boolean;
}

export type TranscriptionSource = "mic" | "speaker";

export interface TranscriptionStatsPayload {
  received: number;
  with_speech: number;
  transcribed: number;
}

export interface TranscriptionEventPayload {
  source: TranscriptionSource;
  text: string;
  raw_text: string;
  confidence: number;
  duration_sec: number;
  processing_time_ms: number;
  audio_level_db: number;
  stats: TranscriptionStatsPayload;
  received_at_ms: number;
}

export const startTranscription = async () => {
  await invoke("start_transcription");
};

export const stopTranscription = async () => {
  await invoke("stop_transcription");
};

export const getTranscriptionStatus = async (): Promise<boolean> => {
  const response = await invoke<RecorderStatusResponse>("transcription_status");
  return Boolean(response?.active);
};

export const listenToTranscription = async (
  handler: (payload: TranscriptionEventPayload) => void
): Promise<UnlistenFn> => {
  const unlisten = await listen<TranscriptionEventPayload>(
    TRANSCRIPTION_EVENT,
    (event) => handler(event.payload)
  );

  return unlisten;
};
