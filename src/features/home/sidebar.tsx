import { Mic, Square } from "lucide-react";
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
  onToggleCapture: () => void;
  activeMeetingId?: string;
  isCapturing: boolean;
  isCaptureBusy?: boolean;
}

export const Sidebar = ({
  meetings,
  onMeetingSelect,
  onToggleCapture,
  activeMeetingId,
  isCapturing,
  isCaptureBusy,
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
        onClick={onToggleCapture}
        disabled={isCaptureBusy}
        className={cn(
          "flex flex-col gap-1 items-center justify-center h-[77px] p-4 border-2 border-dashed rounded-[10px] transition-colors",
          isCapturing
            ? "bg-custom-red text-white border-custom-red"
            : "border-custom-red text-custom-red hover:bg-custom-red/5",
          isCaptureBusy && "opacity-50 cursor-not-allowed"
        )}
      >
        {isCapturing ? (
          <Square className="w-6 h-6" />
        ) : (
          <Mic className="w-6 h-6" />
        )}
        <span className="text-sm font-medium">
          {isCapturing ? "Stop Capture" : "Start Capture"}
        </span>
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
