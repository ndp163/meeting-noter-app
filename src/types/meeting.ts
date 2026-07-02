export interface TranscriptMessage {
  id: string;
  label: "You" | "Speaker";
  timestamp: string;
  content: string;
  source?: "mic" | "speaker";
  isFinal?: boolean;
  sentenceFinal?: boolean;
  committedContent?: string;
  audioOffset?: number;
  /** Realtime translation of `content` (Apple Translation, on-device). */
  translation?: string;
}

export interface DiarizedSegment {
  speakerId: string;
  label: string;
  start: number;
  end: number;
  text: string;
}

/** ASR language a meeting is transcribed in. */
export type MeetingLanguage = "en" | "ja";

export interface Meeting {
  id: string;
  title: string;
  createdAt: number;
  updatedAt: number;
  duration: number;
  status: "recording" | "completed";
  language?: MeetingLanguage;
  /** Meeting app detected at capture start ("Teams", "Zoom", "Chrome", …). */
  platform?: string;
  audioPath?: string;
  transcript: TranscriptMessage[];
  diarization?: DiarizedSegment[];
  summary?: string;
  summaryVi?: string;
}
