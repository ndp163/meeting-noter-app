import { invoke } from "@tauri-apps/api/core";

/** Name of the meeting app currently using the mic, or null. */
export const getMeetingDetectionStatus = async (): Promise<string | null> => {
  return await invoke<string | null>("meeting_detection_status");
};

/**
 * Normalize a raw detected app name into a short platform tag for display.
 * Native apps map to a brand label; browser-based calls (Meet, Zoom web, …)
 * can only be identified as the browser, so they keep the browser name.
 */
export const normalizeMeetingPlatform = (
  raw: string | null,
): string | undefined => {
  if (!raw) return undefined;
  if (raw === "Microsoft Teams") return "Teams";
  return raw;
};

/** Detect the meeting platform in use right now, normalized for display. */
export const detectMeetingPlatform = async (): Promise<string | undefined> => {
  try {
    return normalizeMeetingPlatform(await getMeetingDetectionStatus());
  } catch {
    return undefined;
  }
};
