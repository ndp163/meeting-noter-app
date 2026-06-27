import { invoke } from "@tauri-apps/api/core";
import { listen, UnlistenFn } from "@tauri-apps/api/event";
import { MeetingLanguage } from "@/types/meeting";

const TRANSCRIPTION_EVENT = "transcription://chunk";
const TRANSCRIPTION_STATUS_EVENT = "transcription://status";

export type TranscriptionStatus = "preparing" | "ready";

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
  is_result_final: boolean; // True for final results, false for partial/streaming
  is_sentence_final: boolean; // True when sentence is complete (start new message)
  start_sec: number; // Offset (seconds) of this result within the recording
}

export const startTranscription = async (
  meetingId: string,
  language: MeetingLanguage = "en",
) => {
  await invoke("start_transcription", { meetingId, language });
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

export const listenToTranscriptionStatus = async (
  handler: (status: TranscriptionStatus) => void
): Promise<UnlistenFn> => {
  return await listen<{ status: TranscriptionStatus }>(
    TRANSCRIPTION_STATUS_EVENT,
    (event) => handler(event.payload.status)
  );
};
