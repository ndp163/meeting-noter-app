import { StateCreator } from "zustand";
import { immer } from "zustand/middleware/immer";
import type {
  Meeting,
  TranscriptMessage,
  DiarizedSegment,
} from "@/types/meeting";

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
  setSummary: (meetingId: string, summary: string) => void;
  setDiarization: (meetingId: string, segments: DiarizedSegment[]) => void;
  renameSpeaker: (
    meetingId: string,
    speakerId: string,
    label: string
  ) => void;
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

  setSummary: (meetingId, summary) =>
    set((state) => {
      const meeting = state.meetings.find((m) => m.id === meetingId);
      if (!meeting) return;
      meeting.summary = summary;
      meeting.updatedAt = Date.now();
    }),

  setDiarization: (meetingId, segments) =>
    set((state) => {
      const meeting = state.meetings.find((m) => m.id === meetingId);
      if (!meeting) return;
      meeting.diarization = segments;
      meeting.updatedAt = Date.now();
    }),

  renameSpeaker: (meetingId, speakerId, label) =>
    set((state) => {
      const meeting = state.meetings.find((m) => m.id === meetingId);
      if (!meeting?.diarization) return;
      for (const segment of meeting.diarization) {
        if (segment.speakerId === speakerId) {
          segment.label = label;
        }
      }
      meeting.updatedAt = Date.now();
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
