import { invoke } from "@tauri-apps/api/core";
import { Meeting } from "@/store/meetings.slice";

export const getMeetings = async (): Promise<Meeting[]> => {
  return await invoke<Meeting[]>("get_meetings");
};

export const getMeetingDetail = async (meetingId: string): Promise<Meeting> => {
  return await invoke<Meeting>("get_meeting_detail", { meetingId });
};

export const saveMeeting = async (meeting: Meeting): Promise<void> => {
  await invoke("save_meeting", { meeting });
};

export const deleteMeeting = async (meetingId: string): Promise<void> => {
  await invoke("delete_meeting", { meetingId });
};

export const getMeetingAudioPath = async (
  meetingId: string
): Promise<string> => {
  return await invoke<string>("get_meeting_audio_path", { meetingId });
};

export const createNewMeeting = async (): Promise<Meeting> => {
  const meeting: Meeting = {
    id: crypto.randomUUID(),
    title: "Untitled",
    createdAt: Date.now(),
    updatedAt: Date.now(),
    duration: 0,
    status: "recording",
    transcript: [],
  };
  await saveMeeting(meeting);
  return meeting;
};
