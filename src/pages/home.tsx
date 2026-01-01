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

interface TranscriptMessage {
  id: string;
  speaker: string;
  timestamp: string;
  content: string;
  isUser: boolean;
  source?: string;
  receivedAt?: number; // Add this to track actual timestamp
}

export const HomePage = () => {
  const [activeTab, setActiveTab] = useState<"transcript" | "summary">(
    "transcript"
  );
  const [activeMeetingId, setActiveMeetingId] = useState("1");
  const [messages, setMessages] = useState<TranscriptMessage[]>([]);
  const [isPlaying, setIsPlaying] = useState(false);
  const [isCapturing, setIsCapturing] = useState(false);
  const [isCaptureBusy, setIsCaptureBusy] = useState(false);

  // Ref for auto-scrolling to bottom
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const contentAreaRef = useRef<HTMLDivElement>(null);

  const meetings = [
    { id: "1", title: "Untitled", date: "11:21 12/02/25", isActive: true },
    { id: "2", title: "Untitled", date: "11:21 12/02/25" },
    { id: "3", title: "Untitled", date: "11:21 14/02/25" },
    { id: "4", title: "Untitled", date: "11:21 12/02/25" },
    { id: "5", title: "Untitled", date: "11:21 12/02/25" },
    { id: "6", title: "Untitled", date: "11:21 12/02/25" },
  ];

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

  useEffect(() => {
    let unlisten: (() => void) | null = null;

    listenToTranscription((payload) => {
      console.log(
        "Event received:",
        payload.text,
        "at",
        payload.received_at_ms
      );
      setMessages((prev) => [...prev, mapPayloadToMessage(payload)]);
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
  }, []);

  // Auto-scroll to bottom when new messages arrive
  useEffect(() => {
    if (messages.length > 0 && messagesEndRef.current) {
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
        await stopTranscription();
        setIsCapturing(false);
      } else {
        await startTranscription();
        setIsCapturing(true);
      }
    } catch (error) {
      console.error("Failed to toggle capture", error);
    } finally {
      setIsCaptureBusy(false);
    }
  };

  const handleMeetingSelect = (id: string) => {
    setActiveMeetingId(id);
  };

  return (
    <div className="flex h-screen bg-white">
      {/* Sidebar */}
      <Sidebar
        meetings={meetings}
        activeMeetingId={activeMeetingId}
        onMeetingSelect={handleMeetingSelect}
        onToggleCapture={handleCaptureToggle}
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
                  : "bg-custom-bg-secondary"
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
                      speaker={message.speaker}
                      timestamp={message.timestamp}
                      content={message.content}
                      isUser={message.isUser}
                    />
                  ))}
                  {/* Invisible element to scroll to */}
                  <div ref={messagesEndRef} />
                </>
              ) : (
                <div className="text-custom-text-secondary text-sm">
                  {isCapturing
                    ? "Listening for speech..."
                    : "Press Start Capture to begin transcribing your meeting."}
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
        <AudioPlayer
          currentTime="00:00"
          totalTime="4:43"
          isPlaying={isPlaying}
          onPlayPause={() => setIsPlaying(!isPlaying)}
        />
      </div>
    </div>
  );
};

const mapPayloadToMessage = (
  payload: TranscriptionEventPayload
): TranscriptMessage => {
  const generatedId =
    typeof crypto !== "undefined" && "randomUUID" in crypto
      ? crypto.randomUUID()
      : `${payload.source}-${payload.received_at_ms}-${payload.stats.transcribed}`;

  return {
    id: generatedId,
    speaker: payload.source === "mic" ? "You" : "Speaker",
    timestamp: formatTimestamp(payload.received_at_ms),
    content: payload.text,
    isUser: payload.source === "mic",
    receivedAt: payload.received_at_ms,
    source: payload.source,
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
