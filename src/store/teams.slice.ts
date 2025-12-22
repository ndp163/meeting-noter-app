import { StateCreator } from "zustand";
import { immer } from "zustand/middleware/immer";

export interface MeetingDetail {
  id: string;
  title: string;
  date: string;
  duration: string;
  hasTranscript: boolean;
}

export interface TeamsSlice {
  teams: {
    meetingDetail: MeetingDetail | null;
    setMeetingDetail: (meeting: MeetingDetail | null) => void;
  };
}

export const createTeamsSlice: StateCreator<TeamsSlice, [], [], TeamsSlice> =
  immer((set) => ({
    teams: {
      meetingDetail: null,
      setMeetingDetail: (meeting: MeetingDetail | null) =>
        set((state) => {
          state.teams.meetingDetail = meeting;
        }),
    },
  })) as StateCreator<TeamsSlice, [], [], TeamsSlice>;
