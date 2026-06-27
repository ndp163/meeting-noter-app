import { useEffect, useState, useRef } from "react";
import { listen } from "@tauri-apps/api/event";
import { Sidebar } from "@/features/home/sidebar";
import { MainContent } from "@/features/home/main-content";
import {
  getTranscriptionStatus,
  listenToTranscription,
  listenToTranscriptionStatus,
  startTranscription,
  stopTranscription,
  TranscriptionEventPayload,
} from "@/services/transcription";
import {
  loadMeetings,
  getMeetingDetail,
  saveMeeting,
  deleteMeeting as deleteMeetingService,
  getMeetingAudioPath,
  createNewMeeting,
} from "@/services/meetings";
import { diarizeMeeting } from "@/services/diarization";
import {
  summarizeMeeting,
  translateSummary,
  generateTitle,
  isClaudeAvailable,
} from "@/services/summary";
import { useBoundStore } from "@/store";
import type { TranscriptMessage } from "@/types/meeting";
import type { MainTab } from "@/features/home/main-content";

export const HomePage = () => {
  const [activeTab, setActiveTab] = useState<MainTab>("transcript");
  const [isCapturing, setIsCapturing] = useState(false);
  const [isCaptureBusy, setIsCaptureBusy] = useState(false);
  const [isPreparingModel, setIsPreparingModel] = useState(false);
  const [audioPath, setAudioPath] = useState<string>();
  const [isAutoScroll, setIsAutoScroll] = useState(true);
  const [isDiarizing, setIsDiarizing] = useState(false);
  const [diarizationError, setDiarizationError] = useState<string>();
  const [isSummarizing, setIsSummarizing] = useState(false);
  const [summaryError, setSummaryError] = useState<string>();
  const [isTranslating, setIsTranslating] = useState(false);
  const [translateError, setTranslateError] = useState<string>();
  const [claudeReady, setClaudeReady] = useState<boolean>();
  const [titlingMeetingId, setTitlingMeetingId] = useState<string | null>(null);

  // Zustand store
  const meetings = useBoundStore.use.meetings();
  const currentMeetingId = useBoundStore.use.currentMeetingId();
  const setMeetings = useBoundStore.use.setMeetings();
  const addMeeting = useBoundStore.use.addMeeting();
  const updateMeeting = useBoundStore.use.updateMeeting();
  const deleteMeeting = useBoundStore.use.deleteMeeting();
  const setCurrentMeetingId = useBoundStore.use.setCurrentMeetingId();
  const addTranscriptToMeeting = useBoundStore.use.addTranscriptToMeeting();
  const updateTranscriptInMeeting =
    useBoundStore.use.updateTranscriptInMeeting();
  const setDiarization = useBoundStore.use.setDiarization();
  const setSummary = useBoundStore.use.setSummary();
  const setSummaryTranslation = useBoundStore.use.setSummaryTranslation();
  const renameSpeaker = useBoundStore.use.renameSpeaker();
  const getCurrentMeeting = useBoundStore.use.getCurrentMeeting();
  const getCapturingMeeting = useBoundStore.use.getCapturingMeeting();

  const currentMeeting = getCurrentMeeting();
  const messages = currentMeeting?.transcript || [];
  const canDiarize =
    !!currentMeeting && currentMeeting.status === "completed" && !isCapturing;
  const canSummarize =
    !!currentMeeting &&
    currentMeeting.status === "completed" &&
    !isCapturing &&
    currentMeeting.transcript.length > 0;

  const handleRunDiarization = async () => {
    if (!currentMeetingId || isDiarizing) return;
    setIsDiarizing(true);
    setDiarizationError(undefined);
    try {
      const segments = await diarizeMeeting(currentMeetingId);
      setDiarization(currentMeetingId, segments);
      const meeting = getCurrentMeeting();
      if (meeting) {
        await saveMeeting({ ...meeting, diarization: segments });
      }
    } catch (error) {
      console.error("Diarization failed", error);
      setDiarizationError(String(error));
    } finally {
      setIsDiarizing(false);
    }
  };

  const handleGenerateSummary = async () => {
    if (!currentMeetingId || isSummarizing) return;
    setIsSummarizing(true);
    setSummaryError(undefined);
    try {
      // The backend summarizes from data.json on disk, but the live transcript
      // only lives in the store until something persists it. Flush it first so
      // summarize never reads a stale/empty file while the UI shows a transcript.
      const current = getCurrentMeeting();
      if (current) {
        await saveMeeting(current);
      }
      const summary = await summarizeMeeting(currentMeetingId);
      setSummary(currentMeetingId, summary);
      const meeting = getCurrentMeeting();
      if (meeting) {
        await saveMeeting({ ...meeting, summary });
      }
    } catch (error) {
      console.error("Summary failed", error);
      setSummaryError(String(error));
    } finally {
      setIsSummarizing(false);
    }
  };

  const handleTranslateSummary = async () => {
    if (!currentMeetingId || isTranslating) return;
    setIsTranslating(true);
    setTranslateError(undefined);
    try {
      const summaryVi = await translateSummary(currentMeetingId);
      setSummaryTranslation(currentMeetingId, summaryVi);
      const meeting = getCurrentMeeting();
      if (meeting) {
        await saveMeeting({ ...meeting, summaryVi });
      }
    } catch (error) {
      console.error("Translation failed", error);
      setTranslateError(String(error));
    } finally {
      setIsTranslating(false);
    }
  };

  const handleRenameSpeaker = async (speakerId: string, label: string) => {
    if (!currentMeetingId) return;
    renameSpeaker(currentMeetingId, speakerId, label);
    const meeting = getCurrentMeeting();
    if (meeting) {
      await saveMeeting(meeting);
    }
  };

  // After a recording ends, give still-"Untitled" meetings a generated title.
  // Fire-and-forget: it must not block the capture toggle, and it silently
  // no-ops when Claude is unavailable or there's nothing to title.
  const autoGenerateTitle = async (meetingId: string) => {
    setTitlingMeetingId(meetingId);
    try {
      const title = await generateTitle(meetingId);
      updateMeeting(meetingId, { title });
      const meeting = meetings.find((m) => m.id === meetingId);
      if (meeting) {
        await saveMeeting({ ...meeting, title });
      }
    } catch (error) {
      console.error("Auto title generation failed", error);
    } finally {
      setTitlingMeetingId(null);
    }
  };

  const handleRenameMeeting = async (id: string, title: string) => {
    const meeting = meetings.find((m) => m.id === id);
    if (!meeting || title === meeting.title) return;
    updateMeeting(id, { title });
    await saveMeeting({ ...meeting, title, updatedAt: Date.now() });
  };

  // On opening the Diarization tab, run it once if there's no cached result.
  useEffect(() => {
    if (
      activeTab === "diarization" &&
      canDiarize &&
      !isDiarizing &&
      !currentMeeting?.diarization
    ) {
      void handleRunDiarization();
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [activeTab, currentMeetingId]);

  // Show a "preparing model" state while the ASR engine loads on first record.
  useEffect(() => {
    let unlisten: (() => void) | null = null;
    listenToTranscriptionStatus((status) => {
      setIsPreparingModel(status === "preparing");
    }).then((release) => {
      unlisten = release;
    });
    return () => {
      unlisten?.();
    };
  }, []);

  // Detect the Claude Code CLI once so the Summary tab can degrade gracefully.
  useEffect(() => {
    let mounted = true;
    isClaudeAvailable()
      .then((ready) => mounted && setClaudeReady(ready))
      .catch(() => mounted && setClaudeReady(false));
    return () => {
      mounted = false;
    };
  }, []);

  // Ref for auto-scrolling to bottom
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const contentAreaRef = useRef<HTMLDivElement>(null);

  // Load meetings on mount
  useEffect(() => {
    let mounted = true;

    (async () => {
      try {
        const loadedMeetings = await loadMeetings();
        if (mounted) {
          setMeetings(loadedMeetings);

          // Set current meeting to the most recent one or create a new one
          if (loadedMeetings.length > 0) {
            const mostRecent = loadedMeetings[0];
            setCurrentMeetingId(mostRecent.id);

            // Load audio path if available
            if (mostRecent.audioPath || mostRecent.status === "completed") {
              try {
                const path = await getMeetingAudioPath(mostRecent.id);
                setAudioPath(path);
              } catch (e) {
                console.log("No audio available for this meeting");
              }
            }
          }
        }
      } catch (error) {
        console.error("Failed to load meetings", error);
      }
    })();

    return () => {
      mounted = false;
    };
  }, [setMeetings, setCurrentMeetingId]);

  // Check transcription status on mount
  useEffect(() => {
    let mounted = true;

    (async () => {
      try {
        const active = await getTranscriptionStatus();
        if (mounted) {
          setIsCapturing(active);
        }
      } catch (error) {
        console.error("Failed to fetch recorder status", error);
      }
    })();

    return () => {
      mounted = false;
    };
  }, []);

  // Listen to transcription events
  useEffect(() => {
    let unlisten: (() => void) | null = null;

    listenToTranscription((payload) => {
      console.log(
        "Event received:",
        payload.text,
        "at",
        payload.received_at_ms,
        "is_result_final:",
        payload.is_result_final,
        "is_sentence_final:",
        payload.is_sentence_final,
      );

      const meeting = getCapturingMeeting();
      if (!meeting) return;

      const transcript = meeting.transcript;

      // Find the last message from the SAME source
      let lastIndexOfSource = -1;
      for (let i = transcript.length - 1; i >= 0; i--) {
        if (transcript[i].source === payload.source) {
          lastIndexOfSource = i;
          break;
        }
      }

      const lastMessageOfSource =
        lastIndexOfSource >= 0 ? transcript[lastIndexOfSource] : null;

      // If sentence was finalized (is_sentence_final=true), always create new message
      if (lastMessageOfSource?.sentenceFinal) {
        const newMessage = mapPayloadToMessage(payload);
        addTranscriptToMeeting(meeting.id, newMessage);
        return;
      }

      // Update existing message if: found message from same source AND not sentence-finalized yet
      if (lastMessageOfSource && !lastMessageOfSource.sentenceFinal) {
        const currentMsg = lastMessageOfSource;

        // Determine new content based on is_result_final and committedContent
        let newContent: string;
        let newCommittedContent: string | undefined;

        if (payload.is_result_final && !payload.is_sentence_final) {
          // Batch final (8s) - append new text to committed content
          const committed = currentMsg.committedContent || "";
          newCommittedContent = committed
            ? `${committed} ${payload.text}`
            : payload.text;
          newContent = newCommittedContent;
        } else if (currentMsg.committedContent) {
          // Streaming update AFTER batch final - append to committed content
          newContent = `${currentMsg.committedContent} ${payload.text}`;
          newCommittedContent = currentMsg.committedContent; // Keep committed
        } else {
          // Streaming update before any batch final - just replace
          newContent = payload.text;
          newCommittedContent = undefined;
        }

        updateTranscriptInMeeting(meeting.id, currentMsg.id, {
          content: newContent,
          isFinal: payload.is_result_final,
          sentenceFinal: payload.is_sentence_final,
          committedContent: newCommittedContent,
        });
        return;
      }

      // Create new message (no previous from this source, or last was sentence-finalized)
      const newMessage = mapPayloadToMessage(payload);
      addTranscriptToMeeting(meeting.id, newMessage);
    })
      .then((release) => {
        console.log("Transcription listener registered");
        unlisten = release;
      })
      .catch((error) => {
        console.error("Failed to subscribe to transcription events", error);
      });

    return () => {
      console.log("Cleaning up transcription listener");
      if (unlisten) {
        unlisten();
      }
    };
  }, [
    currentMeetingId,
    getCurrentMeeting,
    addTranscriptToMeeting,
    updateTranscriptInMeeting,
  ]);

  useEffect(() => {
    console.log(!contentAreaRef.current || !messagesEndRef.current);
    if (!contentAreaRef.current || !messagesEndRef.current) return;

    const observer = new IntersectionObserver(
      ([entry]) => {
        console.log("Messages end intersection:", entry.isIntersecting);
        setIsAutoScroll(entry.isIntersecting);
      },
      {
        root: contentAreaRef.current,
        threshold: 0,
      },
    );

    observer.observe(messagesEndRef.current);

    return () => observer.disconnect();
  }, [contentAreaRef.current, messagesEndRef.current]);

  // Auto-scroll to bottom when new messages arrive
  useEffect(() => {
    if (messagesEndRef.current && isAutoScroll) {
      messagesEndRef.current.scrollIntoView({ behavior: "smooth" });
    }
  }, [messages]);

  const handleCaptureToggle = async () => {
    if (isCaptureBusy) {
      return;
    }

    setIsCaptureBusy(true);
    try {
      if (isCapturing) {
        // Stop capture
        await stopTranscription();
        setIsCapturing(false);
        setIsPreparingModel(false);

        // Update meeting status
        if (currentMeetingId) {
          const meeting = getCapturingMeeting();
          updateMeeting(currentMeetingId, { status: "completed" });
          if (meeting) {
            await saveMeeting({ ...meeting, status: "completed" });
            const path = await getMeetingAudioPath(meeting.id);
            setAudioPath(path);

            if (
              claudeReady &&
              meeting.title === "Untitled" &&
              meeting.transcript.length > 0
            ) {
              void autoGenerateTitle(meeting.id);
            }
          }
        }
      } else {
        const newMeeting = await createNewMeeting();
        addMeeting(newMeeting);
        setCurrentMeetingId(newMeeting.id);
        setAudioPath(undefined);
        await startTranscription(newMeeting.id);
        setIsCapturing(true);
      }
    } catch (error) {
      console.error("Failed to toggle capture", error);
    } finally {
      setIsCaptureBusy(false);
    }
  };

  const handleMeetingSelect = async (id: string) => {
    if (isCapturing && id !== currentMeetingId) {
      alert("Please stop the current recording before switching meetings");
      return;
    }
    setCurrentMeetingId(id);

    // Check if meeting already has data in store
    const existingMeeting = meetings.find((m) => m.id === id);
    if (existingMeeting?.transcript && existingMeeting.transcript.length > 0) {
      // Meeting already has data in store, just load audio if needed
      console.log("Meeting already in store, using existing data");
      if (existingMeeting.status === "completed") {
        try {
          const path = await getMeetingAudioPath(id);
          setAudioPath(path);
        } catch (e) {
          console.log("No audio available for this meeting");
          setAudioPath(undefined);
        }
      } else {
        setAudioPath(undefined);
      }
      return;
    }

    // Load full meeting detail from backend only if not in store or empty
    try {
      const meeting = await getMeetingDetail(id);
      updateMeeting(id, meeting);

      // Load audio path if available
      if (meeting.status === "completed") {
        try {
          const path = await getMeetingAudioPath(id);
          setAudioPath(path);
        } catch (e) {
          console.log("No audio available for this meeting");
          setAudioPath(undefined);
        }
      } else {
        setAudioPath(undefined);
      }
    } catch (error) {
      console.error("Failed to load meeting detail", error);
    }
  };

  const handleDeleteMeeting = async (id: string) => {
    try {
      await deleteMeetingService(id);
      deleteMeeting(id);

      if (id === currentMeetingId) {
        setAudioPath(undefined);
        const remainingMeetings = meetings.filter((m) => m.id !== id);
        if (remainingMeetings.length > 0) {
          setCurrentMeetingId(remainingMeetings[0].id);
        } else {
          setCurrentMeetingId(null);
        }
      }
    } catch (error) {
      console.error("Failed to delete meeting", error);
      alert("Failed to delete meeting: " + error);
    }
  };

  // The detector drives recording start/stop via events: the overlay's Start
  // button emits `start-recording-from-alert`, and the meeting ending (mic
  // released, debounced) emits `meeting-ended`.
  useEffect(() => {
    const unlisteners: Array<() => void> = [];

    listen("start-recording-from-alert", () => {
      if (!isCapturing && !isCaptureBusy) {
        void handleCaptureToggle();
      }
    }).then((release) => unlisteners.push(release));

    listen("meeting-ended", () => {
      if (isCapturing && !isCaptureBusy) {
        void handleCaptureToggle();
      }
    }).then((release) => unlisteners.push(release));

    return () => {
      unlisteners.forEach((u) => u());
    };
  }, [isCapturing, isCaptureBusy]);

  return (
    <div className="relative flex h-screen ds-root ds-theme-vintage bg-[var(--ds-bg)]">
      {/* Sidebar */}
      <Sidebar
        meetings={meetings}
        activeMeetingId={currentMeetingId || undefined}
        onMeetingSelect={handleMeetingSelect}
        onToggleCapture={handleCaptureToggle}
        onDeleteMeeting={handleDeleteMeeting}
        onRenameMeeting={handleRenameMeeting}
        isCapturing={isCapturing}
        isCaptureBusy={isCaptureBusy}
        titlingMeetingId={titlingMeetingId}
      />

      {/* Main Content */}
      <MainContent
        activeTab={activeTab}
        setActiveTab={setActiveTab}
        isCapturing={isCapturing}
        isPreparingModel={isPreparingModel}
        messages={messages}
        currentMeetingId={currentMeetingId}
        audioPath={audioPath}
        contentAreaRef={contentAreaRef}
        messagesEndRef={messagesEndRef}
        diarization={currentMeeting?.diarization}
        isDiarizing={isDiarizing}
        diarizationError={diarizationError}
        canDiarize={canDiarize}
        onRunDiarization={handleRunDiarization}
        onRenameSpeaker={handleRenameSpeaker}
        summary={currentMeeting?.summary}
        summaryVi={currentMeeting?.summaryVi}
        isSummarizing={isSummarizing}
        isTranslating={isTranslating}
        summaryError={summaryError}
        translateError={translateError}
        canSummarize={canSummarize}
        claudeReady={claudeReady}
        onRunSummary={handleGenerateSummary}
        onTranslateSummary={handleTranslateSummary}
      />
    </div>
  );
};

const mapPayloadToMessage = (
  payload: TranscriptionEventPayload,
): TranscriptMessage => {
  const generatedId =
    typeof crypto !== "undefined" && "randomUUID" in crypto
      ? crypto.randomUUID()
      : `${payload.source}-${payload.received_at_ms}-${payload.stats.transcribed}`;

  return {
    id: generatedId,
    label: payload.source === "mic" ? "You" : "Speaker",
    timestamp: formatTimestamp(payload.received_at_ms),
    content: payload.text,
    source: payload.source,
    isFinal: payload.is_result_final,
    sentenceFinal: payload.is_sentence_final,
    audioOffset: payload.start_sec,
  };
};

const formatTimestamp = (ms: number) => {
  const date = new Date(ms);
  return date.toLocaleTimeString([], {
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  });
};
