import { invoke } from "@tauri-apps/api/core";
import type { DiarizedSegment } from "@/types/meeting";

export const diarizeMeeting = async (
  meetingId: string
): Promise<DiarizedSegment[]> => {
  return await invoke<DiarizedSegment[]>("diarize_meeting", { meetingId });
};
