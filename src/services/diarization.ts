import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { DiarizedSegment } from "@/types/meeting";

const DIARIZATION_PROGRESS_EVENT = "diarization://progress";

/** Diarization progress for one meeting, fraction 0.0–1.0 across all tracks. */
export interface DiarizationProgress {
  meetingId: string;
  fraction: number;
}

/** Run offline speaker diarization. `numSpeakers` pins the expected number of
 *  remote speakers when known (more accurate clustering); omit for automatic. */
export const diarizeMeeting = async (
  meetingId: string,
  numSpeakers?: number,
): Promise<DiarizedSegment[]> => {
  return await invoke<DiarizedSegment[]>("diarize_meeting", {
    meetingId,
    numSpeakers: numSpeakers ?? null,
  });
};

/** Subscribe to diarization progress events. Returns an unlisten fn. */
export const onDiarizationProgress = (
  handler: (progress: DiarizationProgress) => void,
) =>
  listen<DiarizationProgress>(DIARIZATION_PROGRESS_EVENT, (e) =>
    handler(e.payload),
  );
