import { invoke } from "@tauri-apps/api/core";

/** Name of the meeting app currently using the mic, or null. */
export const getMeetingDetectionStatus = async (): Promise<string | null> => {
  return await invoke<string | null>("meeting_detection_status");
};
