import { StateCreator } from "zustand";
import { immer } from "zustand/middleware/immer";

export interface TranscriptMessage {
  id: string;
  label: "You" | "Speaker";
  timestamp: string;
  content: string;
  source?: "mic" | "speaker";
  isFinal?: boolean;
  sentenceFinal?: boolean;
  committedContent?: string;
}

export interface Meeting {
  id: string;
  title: string;
  createdAt: number;
  updatedAt: number;
  duration: number;
  status: "recording" | "completed";
  audioPath?: string;
  transcript: TranscriptMessage[];
}

export interface MeetingsSlice {
  meetings: Meeting[];
  currentMeetingId: string | null;
  isLoadingMeetings: boolean;

  // Actions
  setMeetings: (meetings: Meeting[]) => void;
  addMeeting: (meeting: Meeting) => void;
  updateMeeting: (id: string, updates: Partial<Meeting>) => void;
  deleteMeeting: (id: string) => void;
  setCurrentMeetingId: (id: string | null) => void;
  addTranscriptToMeeting: (
    meetingId: string,
    message: TranscriptMessage
  ) => void;
  updateTranscriptInMeeting: (
    meetingId: string,
    messageId: string,
    updates: Partial<TranscriptMessage>
  ) => void;
  setIsLoadingMeetings: (isLoading: boolean) => void;
  getCurrentMeeting: () => Meeting | undefined;
  getCapturingMeeting: () => Meeting | undefined;
}

export const createMeetingsSlice: StateCreator<
  MeetingsSlice,
  [],
  [],
  MeetingsSlice
> = immer((set, get) => ({
  meetings: [],
  currentMeetingId: null,
  isLoadingMeetings: false,

  setMeetings: (meetings) =>
    set((state) => {
      for (const meeting of meetings) {
        meeting.transcript = meeting.transcript.map((msg) => ({
          ...msg,
          label: msg.source === "mic" ? "You" : "Speaker",
        }));
      }
      state.meetings = meetings;
    }),

  addMeeting: (meeting) =>
    set((state) => {
      state.meetings = [meeting, ...state.meetings];
    }),

  updateMeeting: (id, updates) =>
    set((state) => {
      state.meetings = state.meetings.map((m) =>
        m.id === id ? { ...m, ...updates, updatedAt: Date.now() } : m
      );
    }),

  deleteMeeting: (id) =>
    set((state) => {
      state.meetings = state.meetings.filter((m) => m.id !== id);
      if (state.currentMeetingId === id) {
        state.currentMeetingId = null;
      }
    }),

  setCurrentMeetingId: (id) => set({ currentMeetingId: id }),

  addTranscriptToMeeting: (meetingId, message) =>
    set((state) => {
      const { meetings } = state;
      const meetingIdx = meetings.findIndex((m) => m.id === meetingId);
      if (meetingIdx === -1) return;
      state.meetings[meetingIdx].transcript.push(message);
      state.meetings[meetingIdx].updatedAt = Date.now();
    }),

  updateTranscriptInMeeting: (meetingId, messageId, updates) =>
    set((state) => {
      const { meetings } = state;
      const meetingIdx = meetings.findIndex((m) => m.id === meetingId);
      if (meetingIdx === -1) return;
      const transcriptIdx = meetings[meetingIdx].transcript.findIndex(
        (msg) => msg.id === messageId
      );
      if (transcriptIdx === -1) return;
      state.meetings[meetingIdx].updatedAt = Date.now();
      state.meetings[meetingIdx].transcript[transcriptIdx] = {
        ...state.meetings[meetingIdx].transcript[transcriptIdx],
        ...updates,
      };
    }),

  setIsLoadingMeetings: (isLoading) =>
    set((state) => {
      state.isLoadingMeetings = isLoading;
    }),

  getCurrentMeeting: () => {
    const state = get();
    return state.meetings.find((m) => m.id === state.currentMeetingId);
  },

  getCapturingMeeting: () => {
    const state = get();
    return state.meetings.find((m) => m.status === "recording");
  },
})) as StateCreator<MeetingsSlice, [], [], MeetingsSlice>;
