import { useState } from "react";
import {
  mockMeetings,
  MeetingSidebar,
  MeetingHeader,
  MeetingTabs,
  MeetingContent,
} from "@/features/teams";
import { useBoundStore } from "@/store";

export const TeamsPage = () => {
  const { meetingDetail } = useBoundStore.use.teams();
  const [activeTab, setActiveTab] = useState<"summary" | "transcript">(
    "summary"
  );
  const [isCapturing, setIsCapturing] = useState(false);

  const handleStartCapture = async () => {
    setIsCapturing(!isCapturing);
  };

  const handleNewMeeting = () => {
    console.log("Create new meeting");
  };

  return (
    <div className="flex h-screen bg-gray-50">
      <MeetingSidebar meetings={mockMeetings} onNewMeeting={handleNewMeeting} />

      <div className="flex-1 flex flex-col">
        <MeetingHeader
          isCapturing={isCapturing}
          onStartCapture={handleStartCapture}
        />

        {meetingDetail && (
          <MeetingTabs activeTab={activeTab} onTabChange={setActiveTab} />
        )}

        <div className="flex-1 overflow-y-auto p-6">
          <MeetingContent activeTab={activeTab} />
        </div>
      </div>
    </div>
  );
};
