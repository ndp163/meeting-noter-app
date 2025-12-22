import { Calendar, Clock, Plus, ChevronRight, ArrowLeft } from "lucide-react";
import { useNavigate } from "react-router";
import { useBoundStore } from "@/store";
import { MeetingDetail } from "@/store/teams.slice";

interface Props {
  meetings: MeetingDetail[];
  onNewMeeting: () => void;
}

export const MeetingSidebar = ({ meetings, onNewMeeting }: Props) => {
  const navigate = useNavigate();
  const { meetingDetail, setMeetingDetail } = useBoundStore.use.teams();

  return (
    <div className="w-80 bg-white border-r border-gray-200 flex flex-col">
      {/* Sidebar Header */}
      <div className="p-4 border-b border-gray-200">
        <div className="flex items-center justify-center gap-3 mb-3 relative">
          <button
            onClick={() => navigate("/")}
            className="p-1.5 hover:bg-gray-100 rounded-lg transition-colors absolute left-0"
            aria-label="Back to home"
          >
            <ArrowLeft className="w-5 h-5 text-gray-600" />
          </button>
          <h2 className="text-lg font-semibold text-gray-900">
            Meeting History
          </h2>
        </div>
      </div>

      {/* Meeting List */}
      <div className="flex-1 overflow-y-auto">
        {meetings.map((meeting) => (
          <button
            key={meeting.id}
            onClick={() => setMeetingDetail(meeting)}
            className={`w-full p-4 border-b border-gray-100 hover:bg-gray-50 transition-colors text-left ${
              meetingDetail?.id === meeting.id
                ? "bg-indigo-50 border-l-4 border-l-indigo-600"
                : ""
            }`}
          >
            <div className="flex items-start justify-between mb-2">
              <h3 className="font-medium text-gray-900 text-sm line-clamp-2">
                {meeting.title}
              </h3>
              <ChevronRight className="w-4 h-4 text-gray-400 shrink-0 ml-2" />
            </div>
            <div className="flex items-center gap-3 text-xs text-gray-500">
              <span className="flex items-center gap-1">
                <Calendar className="w-3 h-3" />
                {new Date(meeting.date).toLocaleDateString("en-US", {
                  month: "short",
                  day: "numeric",
                })}
              </span>
              <span className="flex items-center gap-1">
                <Clock className="w-3 h-3" />
                {meeting.duration}
              </span>
            </div>
          </button>
        ))}
      </div>

      {/* New Meeting Button */}
      <div className="p-4 border-t border-gray-200">
        <button
          onClick={onNewMeeting}
          className="w-full flex items-center justify-center gap-2 bg-cyan-500 text-white py-2.5 rounded-lg hover:bg-fuchsia-600 transition-colors font-medium"
        >
          <Plus className="w-4 h-4" />
          New Meeting
        </button>
      </div>
    </div>
  );
};
