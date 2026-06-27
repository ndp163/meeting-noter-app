import { useState } from "react";
import {
  Mic,
  Square,
  Trash2,
  Pencil,
  ChevronLeft,
  ChevronRight,
  Loader2,
} from "lucide-react";
import { cn } from "@/lib/utils";
import type { Meeting } from "@/types/meeting";
import { useBoundStore } from "@/store";
import { Brand, CaptureButton, IconButton } from "@/design-system";

interface SidebarProps {
  meetings: Meeting[];
  onMeetingSelect: (id: string) => void;
  onToggleCapture: () => void;
  onDeleteMeeting: (id: string) => void;
  onRenameMeeting: (id: string, title: string) => void;
  activeMeetingId?: string;
  isCapturing: boolean;
  isCaptureBusy?: boolean;
  titlingMeetingId?: string | null;
}

export const Sidebar = ({
  meetings,
  onMeetingSelect,
  onToggleCapture,
  onDeleteMeeting,
  onRenameMeeting,
  activeMeetingId,
  isCapturing,
  isCaptureBusy,
  titlingMeetingId,
}: SidebarProps) => {
  const isSidebarCollapsed = useBoundStore.use.isSidebarCollapsed();
  const toggleSidebar = useBoundStore.use.toggleSidebar();

  return (
    <div
      className={cn(
        "flex flex-col gap-2.5 h-screen px-2.5 py-5 border-r border-[var(--ds-border)]",
        isSidebarCollapsed ? "w-16" : "w-full max-w-[411px]",
      )}
      style={{ background: "var(--ds-surface)" }}
    >
      {/* Header */}
      <div
        className={cn(
          "flex items-center h-[78px] py-[23px]",
          isSidebarCollapsed ? "justify-center" : "justify-between",
        )}
      >
        {!isSidebarCollapsed && <Brand />}
        <IconButton
          icon={isSidebarCollapsed ? <ChevronRight className="w-5 h-5" /> : <ChevronLeft className="w-5 h-5" />}
          label={isSidebarCollapsed ? "Expand sidebar" : "Collapse sidebar"}
          onClick={toggleSidebar}
        />
      </div>

      {/* Start Capture */}
      {isSidebarCollapsed ? (
        <button
          onClick={onToggleCapture}
          disabled={isCaptureBusy}
          title={isCapturing ? "Stop Capture" : "Start Capture"}
          className={cn("ds-capture", isCapturing && "ds-capture--active")}
          style={{ height: 52 }}
        >
          {isCapturing ? <Square className="w-6 h-6" /> : <Mic className="w-6 h-6" />}
        </button>
      ) : (
        <CaptureButton
          capturing={isCapturing}
          disabled={isCaptureBusy}
          onClick={onToggleCapture}
        />
      )}

      {/* Meetings List */}
      <div className="flex flex-col gap-1 overflow-y-auto px-1">
        {!isSidebarCollapsed && meetings.length === 0 ? (
          <div className="text-center text-[var(--ds-text-2)] text-sm p-4">
            No meetings yet. Press Start Capture to create one.
          </div>
        ) : (
          meetings.map((meeting) => (
            <MeetingCard
              key={meeting.id}
              meeting={meeting}
              isActive={meeting.id === activeMeetingId}
              onClick={() => onMeetingSelect(meeting.id)}
              onDelete={() => onDeleteMeeting(meeting.id)}
              onRename={(title) => onRenameMeeting(meeting.id, title)}
              isCollapsed={isSidebarCollapsed}
              isTitling={meeting.id === titlingMeetingId}
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
  onRename: (title: string) => void;
  isCollapsed: boolean;
  isTitling?: boolean;
}

const MeetingCard = ({
  meeting,
  isActive,
  onClick,
  onDelete,
  onRename,
  isCollapsed,
  isTitling,
}: MeetingCardProps) => {
  const [isEditing, setIsEditing] = useState(false);
  const [draftTitle, setDraftTitle] = useState(meeting.title);

  const startEditing = (e: React.MouseEvent) => {
    e.stopPropagation();
    setDraftTitle(meeting.title);
    setIsEditing(true);
  };

  const commitEditing = () => {
    const trimmed = draftTitle.trim();
    onRename(trimmed || "Untitled");
    setIsEditing(false);
  };

  const cancelEditing = () => {
    setIsEditing(false);
  };

  const formatDate = (timestamp: number) => {
    const date = new Date(timestamp);
    const hours = date.getHours().toString().padStart(2, "0");
    const minutes = date.getMinutes().toString().padStart(2, "0");
    const day = date.getDate().toString().padStart(2, "0");
    const month = (date.getMonth() + 1).toString().padStart(2, "0");
    const year = date.getFullYear().toString().slice(-2);
    return `${hours}:${minutes} · ${day}/${month}/${year}`;
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
          "flex items-center justify-center min-h-[40px] p-2 rounded-[var(--ds-radius-sm)] cursor-pointer relative",
          isActive ? "bg-[var(--ds-border)]" : "hover:bg-[var(--ds-surface-2)]",
        )}
        title={meeting.title}
      >
        {meeting.status === "recording" ? (
          <div className="w-2 h-2 rounded-full bg-[var(--ds-rec)] animate-pulse" />
        ) : (
          <div className="w-2 h-2 rounded-full bg-[var(--ds-text)]" />
        )}
      </div>
    );
  }

  return (
    <div
      onClick={onClick}
      className={cn("ds-meeting group", isActive && "ds-meeting--active")}
    >
      <div className="ds-meeting__row">
        {isEditing ? (
          <input
            autoFocus
            value={draftTitle}
            onClick={(e) => e.stopPropagation()}
            onChange={(e) => setDraftTitle(e.target.value)}
            onBlur={commitEditing}
            onKeyDown={(e) => {
              if (e.key === "Enter") commitEditing();
              else if (e.key === "Escape") cancelEditing();
            }}
            className="text-base font-medium bg-transparent border-b outline-none w-full"
            style={{ borderColor: "var(--ds-border-2)", color: "var(--ds-text)" }}
          />
        ) : isTitling ? (
          <div className="flex items-center gap-1.5 min-w-0 text-[var(--ds-text-2)]">
            <Loader2 className="w-3.5 h-3.5 animate-spin shrink-0" />
            <span className="ds-meeting__title italic">Generating title…</span>
          </div>
        ) : (
          <div className="flex items-center gap-1.5 min-w-0">
            <p onDoubleClick={startEditing} className="ds-meeting__title">
              {meeting.title}
            </p>
            <button
              onClick={startEditing}
              className="opacity-0 group-hover:opacity-100 p-1 rounded hover:bg-[var(--ds-surface-2)] shrink-0"
              title="Rename meeting"
            >
              <Pencil className="w-3.5 h-3.5 text-[var(--ds-text-2)]" />
            </button>
          </div>
        )}
        {meeting.status === "recording" && (
          <span className="flex items-center gap-1 text-xs text-[var(--ds-rec)]">
            <span className="h-2 w-2 rounded-full bg-[var(--ds-rec)] animate-pulse" />
            Rec
          </span>
        )}
      </div>
      <div className="ds-meeting__row">
        <p className="ds-meeting__meta">{formatDate(meeting.createdAt)}</p>
        {meeting.status !== "recording" && (
          <button
            onClick={handleDelete}
            className="opacity-0 group-hover:opacity-100 p-1 rounded hover:bg-[var(--ds-rec-soft)]"
            title="Delete meeting"
          >
            <Trash2 className="w-4 h-4 text-[var(--ds-rec)]" />
          </button>
        )}
      </div>
    </div>
  );
};
