import { invoke } from "@tauri-apps/api/core";

export const summarizeMeeting = async (
  meetingId: string
): Promise<string> => {
  return await invoke<string>("summarize_meeting", { meetingId });
};

export const isClaudeAvailable = async (): Promise<boolean> => {
  return await invoke<boolean>("claude_available");
};
