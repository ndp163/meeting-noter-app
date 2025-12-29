import { useState } from "react";
import { Sidebar } from "@/features/home/sidebar";
import { Tab } from "@/features/home/tab";
import { Message } from "@/features/home/message";
import { AudioPlayer } from "@/features/home/audio-player";

interface TranscriptMessage {
  id: string;
  speaker: string;
  timestamp: string;
  content: string;
  isUser: boolean;
}

export const HomePage = () => {
  const [activeTab, setActiveTab] = useState<"transcript" | "summary">(
    "transcript"
  );
  const [activeMeetingId, setActiveMeetingId] = useState("1");
  const [isPlaying, setIsPlaying] = useState(false);

  // Mock data
  const meetings = [
    { id: "1", title: "Untitled", date: "11:21 12/02/25", isActive: true },
    { id: "2", title: "Untitled", date: "11:21 12/02/25" },
    { id: "3", title: "Untitled", date: "11:21 14/02/25" },
    { id: "4", title: "Untitled", date: "11:21 12/02/25" },
    { id: "5", title: "Untitled", date: "11:21 12/02/25" },
    { id: "6", title: "Untitled", date: "11:21 12/02/25" },
  ];

  const messages: TranscriptMessage[] = [
    {
      id: "1",
      speaker: "You",
      timestamp: "00:01",
      content: "Hello world!",
      isUser: true,
    },
    {
      id: "2",
      speaker: "Speaker",
      timestamp: "01:04",
      content:
        "Lorem Ipsum is simply dummy text of the printing and typesetting industry. Lorem Ipsum has been the industry's standard dummy text ever since the 1500s",
      isUser: false,
    },
    {
      id: "3",
      speaker: "You",
      timestamp: "00:01",
      content:
        "This handy tool helps you create dummy text for all your layout needs. We are gradually adding new functionality and we welcome your suggestions and feedback.",
      isUser: true,
    },
  ];

  const handleStartCapture = () => {
    console.log("Start capture");
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
        onStartCapture={handleStartCapture}
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
        </div>

        {/* Content Area */}
        <div className="flex-1 border border-custom-bg-primary rounded-[20px] p-5 overflow-y-auto">
          <div className="flex flex-col gap-5">
            {activeTab === "transcript" ? (
              messages.map((message) => (
                <Message
                  key={message.id}
                  speaker={message.speaker}
                  timestamp={message.timestamp}
                  content={message.content}
                  isUser={message.isUser}
                />
              ))
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
