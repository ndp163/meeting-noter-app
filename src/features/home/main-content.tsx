import { Ref } from "react";
import { Tab } from "@/features/home/tab";
import { Message } from "@/features/home/message";
import { AudioPlayer } from "@/features/home/audio-player";
import { DiarizationView } from "@/features/home/diarization-view";
import { SummaryView } from "@/features/home/summary-view";
import { cn } from "@/lib/utils";
import type { TranscriptMessage, DiarizedSegment } from "@/types/meeting";

export type MainTab = "transcript" | "summary" | "diarization";

interface MainContentProps {
  activeTab: MainTab;
  setActiveTab: (tab: MainTab) => void;
  isCapturing: boolean;
  messages: TranscriptMessage[];
  currentMeetingId: string | null;
  audioPath?: string;
  contentAreaRef: Ref<HTMLDivElement>;
  messagesEndRef: Ref<HTMLDivElement>;
  diarization: DiarizedSegment[] | undefined;
  isDiarizing: boolean;
  diarizationError?: string;
  canDiarize: boolean;
  onRunDiarization: () => void;
  onRenameSpeaker: (speakerId: string, label: string) => void;
  summary: string | undefined;
  isSummarizing: boolean;
  summaryError?: string;
  canSummarize: boolean;
  claudeReady: boolean | undefined;
  onRunSummary: () => void;
}

export const MainContent = ({
  activeTab,
  setActiveTab,
  isCapturing,
  messages,
  currentMeetingId,
  audioPath,
  contentAreaRef,
  messagesEndRef,
  diarization,
  isDiarizing,
  diarizationError,
  canDiarize,
  onRunDiarization,
  onRenameSpeaker,
  summary,
  isSummarizing,
  summaryError,
  canSummarize,
  claudeReady,
  onRunSummary,
}: MainContentProps) => {
  return (
    <div className="flex flex-col gap-2.5 flex-1 p-5 overflow-hidden">
      {/* Header with Tabs */}
      <div className="flex gap-2.5 h-[78px] items-center">
        <Tab
          label="Transcript"
          isActive={activeTab === "transcript"}
          onClick={() => setActiveTab("transcript")}
        />
        <Tab
          label="Summary"
          isActive={activeTab === "summary"}
          onClick={() => setActiveTab("summary")}
        />
        <Tab
          label="Diarization"
          isActive={activeTab === "diarization"}
          onClick={() => setActiveTab("diarization")}
        />
        <div className="ml-auto flex items-center gap-2 text-sm text-custom-text-secondary">
          <span
            className={cn(
              "h-2 w-2 rounded-full",
              isCapturing
                ? "bg-custom-red animate-pulse"
                : "bg-custom-bg-secondary",
            )}
          />
          {isCapturing ? "Listening" : "Idle"}
        </div>
      </div>

      {/* Content Area */}
      <div
        ref={contentAreaRef}
        className="flex-1 border border-custom-bg-primary rounded-[20px] p-5 overflow-y-auto"
      >
        <div className="flex flex-col gap-5">
          {activeTab === "transcript" ? (
            messages.length > 0 ? (
              <>
                {messages.map((message) => (
                  <Message
                    key={message.id}
                    label={message.label}
                    timestamp={message.timestamp}
                    content={message.content}
                    isUser={message.source === "mic"}
                  />
                ))}
                {/* Invisible element to scroll to */}
                <div ref={messagesEndRef} />
              </>
            ) : (
              <div className="text-custom-text-secondary text-sm">
                {isCapturing
                  ? "Listening for speech..."
                  : currentMeetingId
                    ? "No transcript yet. Press Start Capture to begin recording."
                    : "Create a new meeting or select an existing one to get started."}
              </div>
            )
          ) : activeTab === "diarization" ? (
            <DiarizationView
              segments={diarization}
              isLoading={isDiarizing}
              error={diarizationError}
              canRun={canDiarize}
              onRun={onRunDiarization}
              onRenameSpeaker={onRenameSpeaker}
            />
          ) : (
            <SummaryView
              summary={summary}
              isLoading={isSummarizing}
              error={summaryError}
              canRun={canSummarize}
              claudeReady={claudeReady}
              onRun={onRunSummary}
            />
          )}
        </div>
      </div>

      {/* Audio Player */}
      <AudioPlayer audioPath={audioPath} />
    </div>
  );
};
