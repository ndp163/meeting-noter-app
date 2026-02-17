import { Mic, Square, Trash2, ChevronLeft, ChevronRight } from "lucide-react";
import { cn } from "@/lib/utils";
import type { Meeting } from "@/store/meetings.slice";
import { useBoundStore } from "@/store";

interface SidebarProps {
  meetings: Meeting[];
  onMeetingSelect: (id: string) => void;
  onToggleCapture: () => void;
  onDeleteMeeting: (id: string) => void;
  activeMeetingId?: string;
  isCapturing: boolean;
  isCaptureBusy?: boolean;
}

export const Sidebar = ({
  meetings,
  onMeetingSelect,
  onToggleCapture,
  onDeleteMeeting,
  activeMeetingId,
  isCapturing,
  isCaptureBusy,
}: SidebarProps) => {
  const isSidebarCollapsed = useBoundStore.use.isSidebarCollapsed();
  const toggleSidebar = useBoundStore.use.toggleSidebar();

  return (
    <div
      className={cn(
        "flex flex-col gap-2.5 h-screen px-2.5 py-5",
        isSidebarCollapsed ? "w-16" : "w-full max-w-[411px]",
      )}
    >
      {/* Header */}
      <div className="flex items-center justify-between h-[78px] px-5 py-[23px]">
        {!isSidebarCollapsed && (
          <h1 className="text-xl font-bold text-custom-text-highlight">
            Meeting Noter
          </h1>
        )}
        <button
          onClick={toggleSidebar}
          className="p-2 hover:bg-gray-100 rounded-lg"
          title={isSidebarCollapsed ? "Expand sidebar" : "Collapse sidebar"}
        >
          {isSidebarCollapsed ? (
            <ChevronRight className="w-5 h-5" />
          ) : (
            <ChevronLeft className="w-5 h-5" />
          )}
        </button>
      </div>

      {/* Start Capture Button */}
      <button
        onClick={onToggleCapture}
        disabled={isCaptureBusy}
        className={cn(
          "flex flex-col gap-1 items-center justify-center h-[77px] p-4 border-2 border-dashed rounded-[10px]",
          isCapturing
            ? "bg-custom-red text-white border-custom-red"
            : "border-custom-red text-custom-red hover:bg-custom-red/5",
          isCaptureBusy && "opacity-50 cursor-not-allowed",
        )}
        title={
          isSidebarCollapsed
            ? isCapturing
              ? "Stop Capture"
              : "Start Capture"
            : undefined
        }
      >
        {isCapturing ? (
          <Square className="w-6 h-6" />
        ) : (
          <Mic className="w-6 h-6" />
        )}
        {!isSidebarCollapsed && (
          <span className="text-sm font-medium">
            {isCapturing ? "Stop Capture" : "Start Capture"}
          </span>
        )}
      </button>

      {/* Meetings List */}
      <div className="flex flex-col gap-2.5 overflow-y-auto px-2">
        {!isSidebarCollapsed && meetings.length === 0 ? (
          <div className="text-center text-custom-text-secondary text-sm p-4">
            No meetings yet. Click + to create a new meeting.
          </div>
        ) : (
          meetings.map((meeting) => (
            <MeetingCard
              key={meeting.id}
              meeting={meeting}
              isActive={meeting.id === activeMeetingId}
              onClick={() => onMeetingSelect(meeting.id)}
              onDelete={() => onDeleteMeeting(meeting.id)}
              isCollapsed={isSidebarCollapsed}
            />
          ))
        )}
      </div>
    </div>
  );
};

interface MeetingCardProps {
  meeting: Meeting;
  isActive: boolean;
  onClick: () => void;
  onDelete: () => void;
  isCollapsed: boolean;
}

const MeetingCard = ({
  meeting,
  isActive,
  onClick,
  onDelete,
  isCollapsed,
}: MeetingCardProps) => {
  const formatDate = (timestamp: number) => {
    const date = new Date(timestamp);
    const hours = date.getHours().toString().padStart(2, "0");
    const minutes = date.getMinutes().toString().padStart(2, "0");
    const day = date.getDate().toString().padStart(2, "0");
    const month = (date.getMonth() + 1).toString().padStart(2, "0");
    const year = date.getFullYear().toString().slice(-2);
    return `${hours}:${minutes} ${day}/${month}/${year}`;
  };

  const handleDelete = (e: React.MouseEvent) => {
    e.stopPropagation();
    onDelete();
  };

  if (isCollapsed) {
    return (
      <div
        onClick={onClick}
        className={cn(
          "flex items-center justify-center min-h-[40px] p-2 rounded-[10px] cursor-pointer relative",
          isActive ? "bg-custom-bg-primary" : "hover:bg-custom-bg-secondary",
        )}
        title={meeting.title}
      >
        {meeting.status === "recording" ? (
          <div className="w-2 h-2 rounded-full bg-custom-red animate-pulse" />
        ) : (
          <div className="w-2 h-2 rounded-full bg-custom-text-primary" />
        )}
      </div>
    );
  }

  return (
    <div
      onClick={onClick}
      className={cn(
        "flex flex-col gap-2.5 items-start justify-center min-h-[79px] p-2.5 rounded-[10px] w-full text-left relative group cursor-pointer",
        isActive ? "bg-custom-bg-primary" : "hover:bg-custom-bg-secondary",
      )}
    >
      <div className="flex items-center justify-between w-full">
        <p className="text-base text-custom-text-primary font-medium">
          {meeting.title}
        </p>
        {meeting.status === "recording" && (
          <span className="flex items-center gap-1 text-xs text-custom-red">
            <span className="h-2 w-2 rounded-full bg-custom-red animate-pulse" />
            Recording
          </span>
        )}
      </div>
      <div className="flex items-center justify-between w-full">
        <p className="text-sm text-custom-text-secondary">
          {formatDate(meeting.createdAt)}
        </p>
        {meeting.status !== "recording" && (
          <button
            onClick={handleDelete}
            className="opacity-0 group-hover:opacity-100 p-1 rounded hover:bg-custom-red/10"
            title="Delete meeting"
          >
            <Trash2 className="w-4 h-4 text-custom-red" />
          </button>
        )}
      </div>
    </div>
  );
};
