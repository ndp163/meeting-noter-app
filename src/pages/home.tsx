import { useEffect, useState, useRef } from "react";
import { listen, emit } from "@tauri-apps/api/event";
import { Sidebar } from "@/features/home/sidebar";
import { MainContent } from "@/features/home/main-content";
import { Toaster } from "@/features/toast/toaster";
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
import { listenToTranslation, getTranslateConfig } from "@/services/translation";
import { detectMeetingPlatform } from "@/services/meeting-detector";
import { showCaption, hideCaption } from "@/services/caption";
import {
  summarizeMeeting,
  translateSummary,
  generateTitle,
  isClaudeAvailable,
  isLocalModelAvailable,
  getSummaryProvider,
  type SummaryProvider,
} from "@/services/summary";
import { useBoundStore } from "@/store";
import { friendlyError } from "@/lib/utils";
import type { TranscriptMessage } from "@/types/meeting";
import type { MainTab } from "@/features/home/main-content";

export const HomePage = () => {
  const [activeTab, setActiveTab] = useState<MainTab>("transcript");
  const [isCapturing, setIsCapturing] = useState(false);
  const [isCaptureBusy, setIsCaptureBusy] = useState(false);
  const [isPreparingModel, setIsPreparingModel] = useState(false);
  const [audioPath, setAudioPath] = useState<string>();
  const [isAutoScroll, setIsAutoScroll] = useState(true);
  const [diarizingMeetingId, setDiarizingMeetingId] = useState<string | null>(null);
  const [diarizationError, setDiarizationError] = useState<string>();
  const [summarizingMeetingId, setSummarizingMeetingId] = useState<string | null>(null);
  const [summaryError, setSummaryError] = useState<string>();
  const [translatingMeetingId, setTranslatingMeetingId] = useState<string | null>(null);
  const [translateError, setTranslateError] = useState<string>();
  // Configured summary-translation target (from Settings › Translation).
  const [translateTarget, setTranslateTarget] = useState<string>("");
  const [claudeReady, setClaudeReady] = useState<boolean>();
  const [localReady, setLocalReady] = useState<boolean>();
  const [summaryProvider, setSummaryProvider] = useState<SummaryProvider>("claude");
  // Readiness of whichever provider is selected.
  const summaryReady =
    summaryProvider === "local" ? localReady : claudeReady;
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
  const setTranslationForSegment =
    useBoundStore.use.setTranslationForSegment();
  const setTranscriptView = useBoundStore.use.setTranscriptView();
  const setDiarization = useBoundStore.use.setDiarization();
  const setSummary = useBoundStore.use.setSummary();
  const setSummaryTranslation = useBoundStore.use.setSummaryTranslation();
  const renameSpeaker = useBoundStore.use.renameSpeaker();
  const getCurrentMeeting = useBoundStore.use.getCurrentMeeting();
  const getCapturingMeeting = useBoundStore.use.getCapturingMeeting();
  const captureLanguage = useBoundStore.use.captureLanguage();
  const openSettings = useBoundStore.use.openSettings();
  const pushToast = useBoundStore.use.pushToast();

  // Derived: only true when the CURRENT meeting is the one being processed.
  const isDiarizing = diarizingMeetingId === currentMeetingId;
  const isSummarizing = summarizingMeetingId === currentMeetingId;
  const isTranslating = translatingMeetingId === currentMeetingId;

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
    if (!currentMeetingId || diarizingMeetingId !== null) return;
    setDiarizingMeetingId(currentMeetingId);
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
      setDiarizationError(
        friendlyError(error, "Speaker identification failed. Please try again."),
      );
    } finally {
      setDiarizingMeetingId(null);
    }
  };

  const handleGenerateSummary = async () => {
    if (!currentMeetingId || summarizingMeetingId !== null) return;
    setSummarizingMeetingId(currentMeetingId);
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
      setSummaryError(
        friendlyError(error, "Couldn't generate the summary. Please try again."),
      );
    } finally {
      setSummarizingMeetingId(null);
    }
  };

  const handleTranslateSummary = async () => {
    if (!currentMeetingId || translatingMeetingId !== null) return;
    if (!translateTarget) {
      pushToast(
        "Choose a translation language in Settings → Translation first.",
        "info",
      );
      return;
    }
    setTranslatingMeetingId(currentMeetingId);
    setTranslateError(undefined);
    try {
      const text = await translateSummary(currentMeetingId, translateTarget);
      setSummaryTranslation(currentMeetingId, translateTarget, text);
      const meeting = getCurrentMeeting();
      if (meeting) {
        await saveMeeting({
          ...meeting,
          summaryTranslation: { lang: translateTarget, text },
        });
      }
    } catch (error) {
      console.error("Translation failed", error);
      setTranslateError(
        friendlyError(error, "Translation failed. Please try again."),
      );
    } finally {
      setTranslatingMeetingId(null);
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
      // Read fresh from the store, not the stale `meetings` closure: by the
      // time the title LLM call returns, the meeting has already transitioned
      // to "completed". Saving a stale snapshot would clobber that status back
      // to "recording", and the orphan purge in loadMeetings then deletes it.
      const meeting = useBoundStore
        .getState()
        .meetings.find((m) => m.id === meetingId);
      if (meeting) {
        await saveMeeting(meeting);
      }
    } catch (error) {
      console.error("Auto title generation failed", error);
    } finally {
      setTitlingMeetingId(null);
    }
  };

  const handleRenameMeeting = async (id: string, title: string) => {
    const existing = useBoundStore.getState().meetings.find((m) => m.id === id);
    if (!existing || title === existing.title) return;
    updateMeeting(id, { title });
    // Persist the fresh store state (correct status + latest transcript),
    // never a stale closure snapshot.
    const meeting = useBoundStore.getState().meetings.find((m) => m.id === id);
    if (meeting) {
      await saveMeeting({ ...meeting, updatedAt: Date.now() });
    }
  };

  // On opening the Diarization tab, run it once if there's no cached result.

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

  // Detect provider + readiness so the Summary tab can degrade gracefully.
  // Re-checked when the settings modal closes, since the user may have switched
  // provider or downloaded the on-device model there.
  const settingsOpen = useBoundStore.use.settingsOpen();
  useEffect(() => {
    if (settingsOpen) return;
    let mounted = true;
    void getSummaryProvider()
      .then((p) => mounted && setSummaryProvider(p))
      .catch(() => {});
    void getTranslateConfig()
      .then((c) => {
        if (!mounted) return;
        setTranscriptView(c.view);
        setTranslateTarget(c.target);
      })
      .catch(() => {});
    isClaudeAvailable()
      .then((ready) => mounted && setClaudeReady(ready))
      .catch(() => mounted && setClaudeReady(false));
    isLocalModelAvailable()
      .then((ready) => mounted && setLocalReady(ready))
      .catch(() => mounted && setLocalReady(false));
    return () => {
      mounted = false;
    };
  }, [settingsOpen]);

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
                // No audio for this meeting — leave the player empty.
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
        unlisten = release;
      })
      .catch((error) => {
        console.error("Failed to subscribe to transcription events", error);
      });

    return () => {
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

  // Realtime translation: attach translated text to its transcript line.
  useEffect(() => {
    let unlisten: (() => void) | null = null;
    listenToTranslation((payload) => {
      const meeting = getCapturingMeeting();
      if (!meeting) return;
      setTranslationForSegment(
        meeting.id,
        payload.source,
        payload.start_sec,
        payload.text,
      );
    })
      .then((release) => {
        unlisten = release;
      })
      .catch((error) => {
        console.error("Failed to subscribe to translation events", error);
      });
    return () => {
      if (unlisten) unlisten();
    };
  }, [getCapturingMeeting, setTranslationForSegment]);

  // Disable auto-scroll only when the user scrolls up away from the bottom.
  // (Don't use marker visibility — appending a line pushes the marker out of
  // view and would wrongly disable auto-scroll, killing it permanently.)
  useEffect(() => {
    const el = contentAreaRef.current;
    if (!el) return;

    const onScroll = () => {
      const nearBottom = el.scrollHeight - el.scrollTop - el.clientHeight < 80;
      setIsAutoScroll(nearBottom);
    };

    el.addEventListener("scroll", onScroll, { passive: true });
    return () => el.removeEventListener("scroll", onScroll);
  }, []);

  // Auto-scroll to bottom when new messages arrive. During capture, updates
  // fire on every ASR token — use instant scroll so smooth animations don't
  // queue up and stutter; reserve smooth for the occasional post-capture case.
  useEffect(() => {
    if (messagesEndRef.current && isAutoScroll) {
      messagesEndRef.current.scrollIntoView({
        behavior: isCapturing ? "auto" : "smooth",
      });
    }
  }, [messages, isAutoScroll, isCapturing]);

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
              summaryReady &&
              meeting.title === "Untitled" &&
              meeting.transcript.length > 0
            ) {
              void autoGenerateTitle(meeting.id);
            }
          }
        }
      } else {
        const platform = await detectMeetingPlatform();
        const newMeeting = await createNewMeeting(captureLanguage, platform);
        addMeeting(newMeeting);
        setCurrentMeetingId(newMeeting.id);
        setAudioPath(undefined);
        await startTranscription(newMeeting.id, captureLanguage);
        setIsCapturing(true);
      }
    } catch (error) {
      console.error("Failed to toggle capture", error);
      setIsCapturing(false);
      setIsPreparingModel(false);
      pushToast(
        "Recording failed — check microphone permissions and try again.",
        "error",
      );
    } finally {
      setIsCaptureBusy(false);
    }
  };

  const handleMeetingSelect = async (id: string) => {
    if (isCapturing && id !== currentMeetingId) {
      pushToast(
        "Stop the current recording before switching meetings.",
        "info",
      );
      return;
    }
    setCurrentMeetingId(id);

    // Check if meeting already has data in store
    const existingMeeting = meetings.find((m) => m.id === id);
    if (existingMeeting?.transcript && existingMeeting.transcript.length > 0) {
      // Meeting already has data in store, just load audio if needed
      if (existingMeeting.status === "completed") {
        try {
          const path = await getMeetingAudioPath(id);
          setAudioPath(path);
        } catch (e) {
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
      pushToast("Couldn't delete the meeting. Please try again.", "error");
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

    // Tray menu's Start/Stop item routes back through the same toggle.
    listen("tray://toggle-recording", () => {
      if (!isCaptureBusy) {
        void handleCaptureToggle();
      }
    }).then((release) => unlisteners.push(release));

    listen("tray://settings", () => {
      openSettings();
    }).then((release) => unlisteners.push(release));

    return () => {
      unlisteners.forEach((u) => u());
    };
  }, [isCapturing, isCaptureBusy]);

  // Mirror recording state into the tray menu (header, Start/Stop, icon).
  useEffect(() => {
    void emit("tray://recording-state", { active: isCapturing });
  }, [isCapturing]);

  // Mirror recording state into the store so any surface (e.g. the update
  // banner) can guard against interrupting an in-progress recording.
  const setRecording = useBoundStore.use.setRecording();
  useEffect(() => {
    setRecording(isCapturing);
  }, [isCapturing, setRecording]);

  // Drive the live-caption overlay: visible only while recording with captions
  // enabled. Reacts to the setting too, so toggling it mid-recording applies.
  const captionEnabled = useBoundStore.use.captionEnabled();
  useEffect(() => {
    void (isCapturing && captionEnabled ? showCaption() : hideCaption());
  }, [isCapturing, captionEnabled]);

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
        summaryTranslation={currentMeeting?.summaryTranslation}
        translateTarget={translateTarget}
        isSummarizing={isSummarizing}
        isTranslating={isTranslating}
        summaryError={summaryError}
        translateError={translateError}
        canSummarize={canSummarize}
        summaryReady={summaryReady}
        summaryProvider={summaryProvider}
        onRunSummary={handleGenerateSummary}
        onTranslateSummary={handleTranslateSummary}
      />

      <Toaster />
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
