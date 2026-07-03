import { Ref, useEffect, useRef, useState } from "react";
import { Loader2, Mic, MessageSquareText } from "lucide-react";
import { Tabs, StatusDot } from "@/design-system";
import { useBoundStore } from "@/store";
import { Message } from "@/features/home/message";
import { AudioPlayer, type AudioPlayerHandle } from "@/features/home/audio-player";
import { DiarizationView } from "@/features/home/diarization-view";
import { SummaryView } from "@/features/home/summary-view";
import type { TranscriptMessage, DiarizedSegment } from "@/types/meeting";
import type { SummaryProvider } from "@/services/summary";

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
  summaryTranslation: { lang: string; text: string } | undefined;
  translateTarget: string;
  isSummarizing: boolean;
  isTranslating: boolean;
  summaryError?: string;
  translateError?: string;
  canSummarize: boolean;
  summaryReady: boolean | undefined;
  summaryProvider: SummaryProvider;
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
  summaryTranslation,
  translateTarget,
  isSummarizing,
  isTranslating,
  summaryError,
  summaryProvider,
  translateError,
  canSummarize,
  summaryReady,
  onRunSummary,
  onTranslateSummary,
}: MainContentProps) => {
  const playerRef = useRef<AudioPlayerHandle>(null);
  const [activeId, setActiveId] = useState<string | null>(null);
  const transcriptView = useBoundStore.use.transcriptView();

  // Map playback position → the last message that has started by then.
  const handleTimeUpdate = (seconds: number) => {
    let id: string | null = null;
    for (const m of messages) {
      if (m.audioOffset !== undefined && m.audioOffset <= seconds) id = m.id;
    }
    setActiveId((prev) => (prev === id ? prev : id));
  };

  // Keep the playing line in view as audio advances.
  useEffect(() => {
    if (!activeId) return;
    const el = document.querySelector(`[data-msg-id="${CSS.escape(activeId)}"]`);
    el?.scrollIntoView({ block: "nearest", behavior: "smooth" });
  }, [activeId]);

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
              <div
                className="flex flex-col gap-2"
                role="log"
                aria-live="polite"
                aria-relevant="additions"
                aria-label="Live transcript"
              >
                {messages.map((message) => {
                  // "translated" shows only the translation (falls back to the
                  // original until it arrives); "both" stacks them; "original"
                  // hides translation.
                  const showTranslated =
                    transcriptView === "translated" && message.translation;
                  const content = showTranslated
                    ? message.translation!
                    : message.content;
                  const translation =
                    transcriptView === "both" ? message.translation : undefined;
                  return (
                    <div key={message.id} data-msg-id={message.id}>
                      <Message
                        label={message.label}
                        timestamp={message.timestamp}
                        content={content}
                        translation={translation}
                        isUser={message.source === "mic"}
                        active={message.id === activeId}
                        pending={isCapturing && message.sentenceFinal === false}
                        onSeek={
                          audioPath && message.audioOffset !== undefined
                            ? () => playerRef.current?.seek(message.audioOffset!)
                            : undefined
                        }
                      />
                    </div>
                  );
                })}
                {/* Invisible element to scroll to */}
                <div ref={messagesEndRef} />
              </div>
            ) : isPreparingModel ? (
              <div className="flex flex-col items-center justify-center gap-4 py-16 text-center">
                <div className="relative flex items-center justify-center">
                  <span className="absolute inline-flex h-14 w-14 rounded-full bg-[var(--ds-accent)] opacity-20 animate-ping" />
                  <span className="relative inline-flex h-14 w-14 items-center justify-center rounded-full bg-[var(--ds-surface-2)]">
                    <Loader2
                      className="h-7 w-7 animate-spin text-[var(--ds-accent)]"
                      strokeWidth={2.5}
                    />
                  </span>
                </div>
                <div className="flex flex-col gap-1">
                  <p className="text-base font-medium text-[var(--ds-text)]">
                    Preparing the speech model…
                  </p>
                  <p className="text-sm text-[var(--ds-text-2)]">
                    This only takes a moment. We&apos;ll start capturing as soon
                    as it&apos;s ready.
                  </p>
                </div>
              </div>
            ) : isCapturing ? (
              <div className="text-[var(--ds-text-2)] text-sm">
                Listening for speech...
              </div>
            ) : (
              <div className="flex flex-col items-center justify-center gap-4 py-16 text-center">
                <span className="inline-flex h-14 w-14 items-center justify-center rounded-full bg-[var(--ds-surface-2)]">
                  {currentMeetingId ? (
                    <Mic className="h-7 w-7 text-[var(--ds-text-2)]" strokeWidth={2} />
                  ) : (
                    <MessageSquareText
                      className="h-7 w-7 text-[var(--ds-text-2)]"
                      strokeWidth={2}
                    />
                  )}
                </span>
                <div className="flex flex-col gap-1">
                  <p className="text-base font-medium text-[var(--ds-text)]">
                    {currentMeetingId
                      ? "No transcript yet"
                      : "No meeting selected"}
                  </p>
                  <p className="text-sm text-[var(--ds-text-2)]">
                    {currentMeetingId
                      ? "Press Start Capture in the sidebar to begin recording."
                      : "Create a new meeting or pick one from the sidebar to get started."}
                  </p>
                </div>
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
              summaryTranslation={summaryTranslation}
              translateTarget={translateTarget}
              isLoading={isSummarizing}
              isTranslating={isTranslating}
              error={summaryError}
              translateError={translateError}
              canRun={canSummarize}
              summaryReady={summaryReady}
              summaryProvider={summaryProvider}
              onRun={onRunSummary}
              onTranslate={onTranslateSummary}
            />
          )}
        </div>
      </div>

      {/* Audio Player */}
      <AudioPlayer
        ref={playerRef}
        audioPath={audioPath}
        onTimeUpdate={handleTimeUpdate}
      />
    </div>
  );
};
