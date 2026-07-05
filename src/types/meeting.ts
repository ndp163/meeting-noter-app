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
  /** Per-word time spans for word-level seek. Absent on meetings diarized
   *  before words were recorded. */
  words?: DiarizedWord[];
}

/** One word with its exact time span inside a diarized segment. */
export interface DiarizedWord {
  text: string;
  start: number;
  end: number;
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
  /** On-demand translation of `summary`, keyed by the target language it was
   *  produced for (from the Translation settings target). Invalidated when the
   *  summary is regenerated. */
  summaryTranslation?: { lang: string; text: string };
}
