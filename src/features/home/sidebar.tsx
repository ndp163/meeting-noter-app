import { Mic } from "lucide-react";
import { cn } from "@/lib/utils";

interface Meeting {
  id: string;
  title: string;
  date: string;
  isActive?: boolean;
}

interface SidebarProps {
  meetings: Meeting[];
  onMeetingSelect: (id: string) => void;
  onStartCapture: () => void;
  activeMeetingId?: string;
}

export const Sidebar = ({
  meetings,
  onMeetingSelect,
  onStartCapture,
  activeMeetingId,
}: SidebarProps) => {
  return (
    <div className="flex flex-col gap-2.5 w-full max-w-[411px] h-screen px-2.5 py-5">
      {/* Header */}
      <div className="flex items-center justify-center h-[78px] px-[88px] py-[23px]">
        <h1 className="text-xl font-bold text-custom-text-highlight">
          Meeting Noter
        </h1>
      </div>

      {/* Start Capture Button */}
      <button
        onClick={onStartCapture}
        className="flex items-center justify-center h-[77px] px-[166px] py-[23px] border-2 border-dashed border-custom-red rounded-[10px] hover:bg-custom-red/5 transition-colors"
      >
        <Mic className="w-10 h-10 text-custom-red" />
      </button>

      {/* Meetings List */}
      <div className="flex flex-col gap-2.5 overflow-y-auto">
        {meetings.map((meeting) => (
          <MeetingCard
            key={meeting.id}
            meeting={meeting}
            isActive={meeting.id === activeMeetingId}
            onClick={() => onMeetingSelect(meeting.id)}
          />
        ))}
      </div>
    </div>
  );
};

interface MeetingCardProps {
  meeting: Meeting;
  isActive: boolean;
  onClick: () => void;
}

const MeetingCard = ({ meeting, isActive, onClick }: MeetingCardProps) => {
  return (
    <button
      onClick={onClick}
      className={cn(
        "flex flex-col gap-2.5 items-start justify-center h-[79px] p-2.5 rounded-[10px] w-full text-left transition-colors",
        isActive ? "bg-custom-bg-primary" : "hover:bg-custom-bg-secondary"
      )}
    >
      <div className="flex items-center justify-center">
        <p className="text-base text-custom-text-primary">{meeting.title}</p>
      </div>
      <p className="text-sm text-custom-text-secondary">{meeting.date}</p>
    </button>
  );
};
