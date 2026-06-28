import { invoke } from "@tauri-apps/api/core";
import { Meeting, MeetingLanguage } from "@/types/meeting";

export const getMeetings = async (): Promise<Meeting[]> => {
  return await invoke<Meeting[]>("get_meetings");
};

// Meetings still marked "recording" on load are orphans from a session that
// never finished (app crashed/closed mid-recording, or a stale save clobbered
// the status). They may still hold a real transcript + audio on disk, so never
// delete blindly: recover any orphan that has data (flip it to "completed"),
// and only discard the truly empty ones (no transcript, no audio).
export const loadMeetings = async (): Promise<Meeting[]> => {
  const meetings = await getMeetings();
  const orphans = meetings.filter((m) => m.status === "recording");

  await Promise.all(
    orphans.map(async (m) => {
      const hasTranscript = m.transcript.length > 0;
      const hasAudio = await getMeetingAudioPath(m.id)
        .then(() => true)
        .catch(() => false);
      if (hasTranscript || hasAudio) {
        m.status = "completed";
        await saveMeeting(m);
      } else {
        await deleteMeeting(m.id);
      }
    }),
  );

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

export const createNewMeeting = async (
  language: MeetingLanguage = "en",
): Promise<Meeting> => {
  const meeting: Meeting = {
    id: crypto.randomUUID(),
    title: "Untitled",
    createdAt: Date.now(),
    updatedAt: Date.now(),
    duration: 0,
    status: "recording",
    language,
    transcript: [],
  };
  await saveMeeting(meeting);
  return meeting;
};
