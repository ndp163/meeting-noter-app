import { Ref, useRef } from "react";
import { Tabs, StatusDot } from "@/design-system";
import { Message } from "@/features/home/message";
import { AudioPlayer, type AudioPlayerHandle } from "@/features/home/audio-player";
import { DiarizationView } from "@/features/home/diarization-view";
import { SummaryView } from "@/features/home/summary-view";
import type { TranscriptMessage, DiarizedSegment } from "@/types/meeting";

export type MainTab = "transcript" | "summary" | "diarization";

interface MainContentProps {
  activeTab: MainTab;
  setActiveTab: (tab: MainTab) => void;
  isCapturing: boolean;
  isPreparingModel: boolean;
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
  summaryVi: string | undefined;
  isSummarizing: boolean;
  isTranslating: boolean;
  summaryError?: string;
  translateError?: string;
  canSummarize: boolean;
  claudeReady: boolean | undefined;
  onRunSummary: () => void;
  onTranslateSummary: () => void;
}

export const MainContent = ({
  activeTab,
  setActiveTab,
  isCapturing,
  isPreparingModel,
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
  summaryVi,
  isSummarizing,
  isTranslating,
  summaryError,
  translateError,
  canSummarize,
  claudeReady,
  onRunSummary,
  onTranslateSummary,
}: MainContentProps) => {
  const playerRef = useRef<AudioPlayerHandle>(null);
  return (
    <div className="flex flex-col gap-2.5 flex-1 p-5 overflow-hidden">
      {/* Header with Tabs */}
      <div className="flex gap-2.5 h-[78px] items-center">
        <Tabs
          items={[
            { id: "transcript", label: "Transcript" },
            { id: "summary", label: "Summary" },
            { id: "diarization", label: "Diarization" },
          ]}
          value={activeTab}
          onChange={(id) => setActiveTab(id as MainTab)}
        />
        <div className="ml-auto">
          <StatusDot
            tone={isCapturing ? "rec" : "idle"}
            pulse={isPreparingModel || isCapturing}
            label={
              isPreparingModel
                ? "Preparing model…"
                : isCapturing
                  ? "Listening"
                  : "Idle"
            }
          />
        </div>
      </div>

      {/* Content Area */}
      <div
        ref={contentAreaRef}
        className="ds-card flex-1 p-5 overflow-y-auto"
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
                    onSeek={
                      audioPath && message.audioOffset !== undefined
                        ? () => playerRef.current?.seek(message.audioOffset!)
                        : undefined
                    }
                  />
                ))}
                {/* Invisible element to scroll to */}
                <div ref={messagesEndRef} />
              </>
            ) : (
              <div className="text-custom-text-secondary text-sm">
                {isPreparingModel
                  ? "Preparing the speech model, this only takes a moment…"
                  : isCapturing
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
              onSeek={(seconds) => playerRef.current?.seek(seconds)}
            />
          ) : (
            <SummaryView
              summary={summary}
              summaryVi={summaryVi}
              isLoading={isSummarizing}
              isTranslating={isTranslating}
              error={summaryError}
              translateError={translateError}
              canRun={canSummarize}
              claudeReady={claudeReady}
              onRun={onRunSummary}
              onTranslate={onTranslateSummary}
            />
          )}
        </div>
      </div>

      {/* Audio Player */}
      <AudioPlayer ref={playerRef} audioPath={audioPath} />
    </div>
  );
};
