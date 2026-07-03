import { useCallback, useEffect, useState } from "react";
import {
  Mic,
  Square,
  Trash2,
  Pencil,
  ChevronLeft,
  ChevronRight,
  Loader2,
  Settings,
  Download,
  Check,
} from "lucide-react";
import { cn, formatBytes } from "@/lib/utils";
import { fetchModelSizes } from "@/services/setup";
import type { Meeting, MeetingLanguage } from "@/types/meeting";
import { useBoundStore } from "@/store";
import { SIDEBAR_COLLAPSE_AT } from "@/store/ui.slice";
import {
  Brand,
  CaptureButton,
  ConfirmDialog,
  IconButton,
  Select,
} from "@/design-system";
import { SettingsModal } from "@/features/settings/settings-modal";
import { LANGUAGES, languageInfo } from "@/lib/languages";

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
  const sidebarWidth = useBoundStore.use.sidebarWidth();
  const setSidebarWidth = useBoundStore.use.setSidebarWidth();
  const setSidebarCollapsed = useBoundStore.use.setSidebarCollapsed();
  const captureLanguage = useBoundStore.use.captureLanguage();
  const setCaptureLanguage = useBoundStore.use.setCaptureLanguage();
  const installedLanguages = useBoundStore.use.installedLanguages();
  const refreshInstalledLanguages =
    useBoundStore.use.refreshInstalledLanguages();
  const settingsOpen = useBoundStore.use.settingsOpen();
  const openSettings = useBoundStore.use.openSettings();
  const closeSettings = useBoundStore.use.closeSettings();

  useEffect(() => {
    void refreshInstalledLanguages();
  }, [refreshInstalledLanguages]);

  // Real per-language download sizes from the manifest (bytes). Best-effort:
  // stays empty offline, and the picker then shows just a download icon.
  const [modelSizes, setModelSizes] = useState<
    Partial<Record<MeetingLanguage, number>>
  >({});
  useEffect(() => {
    void fetchModelSizes().then((s) => s && setModelSizes(s));
  }, []);

  // `null` while loading — treat as installed so we don't flash a prompt.
  const langReady =
    installedLanguages === null || installedLanguages.includes(captureLanguage);

  // Block capture when the selected language isn't installed: open the Models
  // panel to download it instead of silently failing.
  const handleToggleCapture = () => {
    if (!isCapturing && !langReady) {
      openSettings("models");
      return;
    }
    onToggleCapture();
  };

  // Meeting queued for deletion — non-null shows the confirm dialog.
  const [pendingDelete, setPendingDelete] = useState<Meeting | null>(null);

  // Drag the right edge to resize. Dragging narrower than SIDEBAR_COLLAPSE_AT
  // snaps to the collapsed icon rail; committed width is clamped in the store.
  const [isResizing, setIsResizing] = useState(false);
  const startResize = useCallback(
    (e: React.PointerEvent) => {
      e.preventDefault();
      setIsResizing(true);
      const onMove = (ev: PointerEvent) => {
        const w = ev.clientX;
        if (w < SIDEBAR_COLLAPSE_AT) {
          setSidebarCollapsed(true);
        } else {
          if (isSidebarCollapsed) setSidebarCollapsed(false);
          setSidebarWidth(w);
        }
      };
      const onUp = () => {
        setIsResizing(false);
        window.removeEventListener("pointermove", onMove);
        window.removeEventListener("pointerup", onUp);
      };
      window.addEventListener("pointermove", onMove);
      window.addEventListener("pointerup", onUp);
    },
    [isSidebarCollapsed, setSidebarCollapsed, setSidebarWidth],
  );

  return (
    <div
      className={cn(
        "relative flex flex-col gap-2.5 h-screen px-2.5 py-5 border-r border-[var(--ds-border)] shrink-0",
        isResizing ? "select-none" : "transition-[width] duration-150",
      )}
      style={{
        width: isSidebarCollapsed ? 64 : sidebarWidth,
        background: "var(--ds-surface)",
      }}
    >
      {/* Header */}
      <div
        className={cn(
          "flex items-center h-[78px] py-[23px]",
          isSidebarCollapsed ? "justify-center" : "justify-between",
        )}
      >
        {!isSidebarCollapsed && <Brand />}
        <div className="flex items-center gap-1">
          {!isSidebarCollapsed && (
            <IconButton
              icon={<Settings className="w-5 h-5" />}
              label="Settings"
              onClick={() => openSettings()}
            />
          )}
          <IconButton
            icon={isSidebarCollapsed ? <ChevronRight className="w-5 h-5" /> : <ChevronLeft className="w-5 h-5" />}
            label={isSidebarCollapsed ? "Expand sidebar" : "Collapse sidebar"}
            onClick={toggleSidebar}
          />
        </div>
      </div>

      {/* Start Capture */}
      {isSidebarCollapsed ? (
        <button
          onClick={handleToggleCapture}
          disabled={isCaptureBusy}
          title={isCapturing ? "Stop Capture" : "Start Capture"}
          className={cn("ds-capture shrink-0", isCapturing && "ds-capture--active")}
          style={{ height: 52 }}
        >
          {isCapturing ? <Square className="w-6 h-6" /> : <Mic className="w-6 h-6" />}
        </button>
      ) : (
        <CaptureButton
          capturing={isCapturing}
          disabled={isCaptureBusy}
          onClick={handleToggleCapture}
          className="shrink-0"
        />
      )}

      {/* Transcription language — locked while a recording is in progress. */}
      {!isSidebarCollapsed && (
        <div
          title={isCapturing ? "Stop recording to change language" : undefined}
        >
          <Select
            label="Transcribe language"
            disabled={isCapturing}
            value={captureLanguage}
            onChange={(id) => setCaptureLanguage(id as "en" | "ja")}
            options={LANGUAGES.map((l) => {
              // `null` while loading — assume installed so we don't flash a badge.
              const installed =
                installedLanguages === null ||
                installedLanguages.includes(l.id);
              return {
                id: l.id,
                label: l.native,
                hint: l.label,
                trailing: installed ? (
                  <Check className="w-4 h-4 text-[var(--ds-ok)]" />
                ) : (
                  <>
                    <Download className="w-3.5 h-3.5" />
                    {modelSizes[l.id] ? formatBytes(modelSizes[l.id]!) : null}
                  </>
                ),
              };
            })}
          />
          {!langReady && !isCapturing && (
            <button
              onClick={() => openSettings("models")}
              className="flex items-center gap-1.5 self-start text-xs text-[var(--ds-accent)] hover:underline px-1 pt-2"
            >
              <Download className="w-3.5 h-3.5" />
              {languageInfo(captureLanguage).label} model not installed — download
            </button>
          )}
        </div>
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
              onDelete={() => setPendingDelete(meeting)}
              onRename={(title) => onRenameMeeting(meeting.id, title)}
              isCollapsed={isSidebarCollapsed}
              isTitling={meeting.id === titlingMeetingId}
            />
          ))
        )}
      </div>

      <SettingsModal open={settingsOpen} onClose={closeSettings} />

      {/* Delete confirmation — guards against accidental removal. */}
      <ConfirmDialog
        open={pendingDelete !== null}
        title="Delete meeting?"
        message={
          <>
            “{pendingDelete?.title}” and its transcript will be permanently
            deleted. This can’t be undone.
          </>
        }
        confirmLabel="Delete"
        confirmVariant="danger"
        confirmIcon={<Trash2 className="w-4 h-4" />}
        onConfirm={() => {
          if (pendingDelete) onDeleteMeeting(pendingDelete.id);
          setPendingDelete(null);
        }}
        onCancel={() => setPendingDelete(null)}
      />

      {/* Resize handle — hidden in the collapsed rail (drag to expand instead). */}
      {!isSidebarCollapsed && (
        <div
          onPointerDown={startResize}
          onDoubleClick={toggleSidebar}
          title="Drag to resize · double-click to collapse"
          className={cn(
            "absolute top-0 right-0 h-full w-1.5 translate-x-1/2 cursor-col-resize z-10",
            "transition-colors duration-150",
            // Fainter tint on hover, with a short delay so a passing cursor
            // doesn't flash it; full accent only while actively dragging.
            "hover:delay-100 hover:bg-[color-mix(in_srgb,var(--ds-accent)_30%,transparent)]",
            isResizing && "bg-[var(--ds-accent)]",
          )}
        />
      )}
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
        role="button"
        tabIndex={0}
        aria-current={isActive ? "true" : undefined}
        aria-label={meeting.title}
        onKeyDown={(e) => {
          if (e.key === "Enter" || e.key === " ") {
            e.preventDefault();
            onClick();
          }
        }}
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
      role="button"
      tabIndex={0}
      aria-current={isActive ? "true" : undefined}
      onKeyDown={(e) => {
        // Ignore keys while editing the title inline.
        if (isEditing) return;
        if (e.key === "Enter" || e.key === " ") {
          e.preventDefault();
          onClick();
        }
      }}
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
              className="opacity-0 group-hover:opacity-100 p-1 rounded shrink-0 transition-colors hover:bg-[var(--ds-border-2)]"
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
        <div className="flex items-center gap-1.5 min-w-0">
          <p className="ds-meeting__meta">{formatDate(meeting.createdAt)}</p>
          {meeting.platform && (
            <span className="ds-src shrink-0">{meeting.platform}</span>
          )}
        </div>
        {meeting.status !== "recording" && (
          <button
            onClick={handleDelete}
            className="opacity-0 group-hover:opacity-100 p-1 rounded shrink-0 transition-colors hover:bg-[var(--ds-border-2)]"
            title="Delete meeting"
          >
            <Trash2 className="w-4 h-4 text-[var(--ds-rec)]" />
          </button>
        )}
      </div>
    </div>
  );
};
