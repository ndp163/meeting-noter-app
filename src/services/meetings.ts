import { invoke } from "@tauri-apps/api/core";
import { Meeting } from "@/types/meeting";

export const getMeetings = async (): Promise<Meeting[]> => {
  return await invoke<Meeting[]>("get_meetings");
};

// Meetings still marked "recording" on load are orphans from a session that
// never finished (app crashed/closed mid-recording). Delete them and return
// the clean list.
export const loadMeetings = async (): Promise<Meeting[]> => {
  const meetings = await getMeetings();
  const orphans = meetings.filter((m) => m.status === "recording");
  await Promise.all(orphans.map((m) => deleteMeeting(m.id)));
  return meetings.filter((m) => m.status !== "recording");
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
