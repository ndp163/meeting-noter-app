import { useEffect, useState, useRef } from "react";
import { Sidebar } from "@/features/home/sidebar";
import { Tab } from "@/features/home/tab";
import { Message } from "@/features/home/message";
import { AudioPlayer } from "@/features/home/audio-player";
import { cn } from "@/lib/utils";
import {
  getTranscriptionStatus,
  listenToTranscription,
  startTranscription,
  stopTranscription,
  TranscriptionEventPayload,
} from "@/services/transcription";
import {
  getMeetings,
  getMeetingDetail,
  saveMeeting,
  deleteMeeting as deleteMeetingService,
  getMeetingAudioPath,
  createNewMeeting,
} from "@/services/meetings";
import { useBoundStore } from "@/store";
import type { TranscriptMessage } from "@/store/meetings.slice";

export const HomePage = () => {
  const [activeTab, setActiveTab] = useState<"transcript" | "summary">(
    "transcript",
  );
  const [isCapturing, setIsCapturing] = useState(false);
  const [isCaptureBusy, setIsCaptureBusy] = useState(false);
  const [audioPath, setAudioPath] = useState<string>();
  const [isAutoScroll, setIsAutoScroll] = useState(true);

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
  const getCurrentMeeting = useBoundStore.use.getCurrentMeeting();
  const getCapturingMeeting = useBoundStore.use.getCapturingMeeting();

  const currentMeeting = getCurrentMeeting();
  const messages = currentMeeting?.transcript || [];

  // Ref for auto-scrolling to bottom
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const contentAreaRef = useRef<HTMLDivElement>(null);

  // Load meetings on mount
  useEffect(() => {
    let mounted = true;

    (async () => {
      try {
        const loadedMeetings = await getMeetings();
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

        // Update meeting status
        if (currentMeetingId) {
          const meeting = getCapturingMeeting();
          updateMeeting(currentMeetingId, { status: "completed" });
          if (meeting) {
            await saveMeeting({ ...meeting, status: "completed" });
            const path = await getMeetingAudioPath(meeting.id);
            setAudioPath(path);
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

  return (
    <div className="flex h-screen bg-white">
      {/* Sidebar */}
      <Sidebar
        meetings={meetings}
        activeMeetingId={currentMeetingId || undefined}
        onMeetingSelect={handleMeetingSelect}
        onToggleCapture={handleCaptureToggle}
        onDeleteMeeting={handleDeleteMeeting}
        isCapturing={isCapturing}
        isCaptureBusy={isCaptureBusy}
      />

      {/* Main Content */}
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
            ) : (
              <div className="text-custom-text-primary">
                <p>Summary content will be displayed here.</p>
              </div>
            )}
          </div>
        </div>

        {/* Audio Player */}
        <AudioPlayer audioPath={audioPath} />
      </div>
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
