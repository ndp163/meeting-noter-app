import { MeetingDetail } from "@/store/teams.slice";

export const mockMeetings: MeetingDetail[] = [
  {
    id: "1",
    title: "Q4 Planning Session",
    date: "2024-12-05",
    duration: "1h 23m",
    hasTranscript: true,
  },
  {
    id: "2",
    title: "Product Review Meeting",
    date: "2024-12-04",
    duration: "45m",
    hasTranscript: true,
  },
  {
    id: "3",
    title: "Team Standup",
    date: "2024-12-03",
    duration: "15m",
    hasTranscript: true,
  },
  {
    id: "4",
    title: "Client Presentation",
    date: "2024-12-02",
    duration: "2h 10m",
    hasTranscript: true,
  },
  {
    id: "5",
    title: "Sprint Retrospective",
    date: "2024-12-01",
    duration: "1h 5m",
    hasTranscript: true,
  },
];
