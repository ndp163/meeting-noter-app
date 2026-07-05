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
  closeTranscriptMessage: (meetingId: string, messageId: string) => void;
  setTranslationForSegment: (
    meetingId: string,
    source: "mic" | "speaker",
    startSec: number,
    translation: string
  ) => void;
  setIsLoadingMeetings: (isLoading: boolean) => void;
  setSummary: (meetingId: string, summary: string) => void;
  setSummaryTranslation: (
    meetingId: string,
    lang: string,
    text: string,
  ) => void;
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

  // Close an open (not sentence-final) message that was cut off by the other
  // stream's interjection. Its interim tail is superseded by the payload that
  // starts the next message, so freeze it at the committed text — or drop the
  // message entirely when nothing was committed yet (its text re-appears in
  // the new message below, keeping the timeline chronological).
  closeTranscriptMessage: (meetingId, messageId) =>
    set((state) => {
      const meeting = state.meetings.find((m) => m.id === meetingId);
      if (!meeting) return;
      const idx = meeting.transcript.findIndex((msg) => msg.id === messageId);
      if (idx === -1) return;
      const msg = meeting.transcript[idx];
      if (msg.sentenceFinal) return;
      if (msg.committedContent) {
        msg.content = msg.committedContent;
        msg.isFinal = true;
        msg.sentenceFinal = true;
      } else {
        meeting.transcript.splice(idx, 1);
      }
      meeting.updatedAt = Date.now();
    }),

  // Match a translation to its transcript line by source + start offset (the
  // backend keys `translation://chunk` on the message's start_sec / audioOffset).
  setTranslationForSegment: (meetingId, source, startSec, translation) =>
    set((state) => {
      const meeting = state.meetings.find((m) => m.id === meetingId);
      if (!meeting) return;
      // Latest matching message wins (start offsets are unique per message).
      for (let i = meeting.transcript.length - 1; i >= 0; i--) {
        const msg = meeting.transcript[i];
        if (
          msg.source === source &&
          msg.audioOffset !== undefined &&
          Math.abs(msg.audioOffset - startSec) < 0.001
        ) {
          msg.translation = translation;
          return;
        }
      }
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
      // A new summary invalidates any prior translation.
      meeting.summaryTranslation = undefined;
      meeting.updatedAt = Date.now();
    }),

  setSummaryTranslation: (meetingId, lang, text) =>
    set((state) => {
      const meeting = state.meetings.find((m) => m.id === meetingId);
      if (!meeting) return;
      meeting.summaryTranslation = { lang, text };
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
