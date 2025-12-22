import { useBoundStore } from "@/store";
import { EmptyMeetingState } from "./EmptyMeetingState";
import { MeetingSummary } from "./MeetingSummary";
import { MeetingTranscript } from "./MeetingTranscript";

interface Props {
  activeTab: "summary" | "transcript";
}

export const MeetingContent = ({ activeTab }: Props) => {
  const { meetingDetail } = useBoundStore.use.teams();
  return meetingDetail ? (
    activeTab === "summary" ? (
      <MeetingSummary />
    ) : (
      <MeetingTranscript />
    )
  ) : (
    <EmptyMeetingState />
  );
};
