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

export interface DiarizedSegment {
  speakerId: string;
  label: string;
  start: number;
  end: number;
  text: string;
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
  diarization?: DiarizedSegment[];
  summary?: string;
}
