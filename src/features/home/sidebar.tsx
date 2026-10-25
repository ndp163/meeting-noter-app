import { useCallback, useEffect, useRef, useState } from "react";
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
  X,
} from "lucide-react";
import { cn, formatBytes } from "@/lib/utils";
import { fetchModelSizes } from "@/services/setup";
import type { Meeting, MeetingLanguage } from "@/types/meeting";
import { useBoundStore } from "@/store";
import { SIDEBAR_COLLAPSE_AT } from "@/store/ui.slice";
import {
  Brand,
  Button,
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
  onDeleteMeetings: (ids: string[]) => void;
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
  onDeleteMeetings,
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

  // Meetings queued for deletion — non-null shows the confirm dialog.
  const [pendingDelete, setPendingDelete] = useState<Meeting[] | null>(null);

  // Multi-select, Finder-style: hover checkbox or ⌘-click toggles, ⇧-click
  // selects a range. While anything is selected a plain click toggles too, and
  // a bottom bar offers bulk delete. In-progress recordings are never selectable.
  const [selectedIds, setSelectedIds] = useState<Set<string>>(new Set());
  const anchorId = useRef<string | null>(null);
  const selectable = meetings.filter((m) => m.status !== "recording");
  const isSelecting = selectedIds.size > 0;
  const clearSelection = () => {
    setSelectedIds(new Set());
    anchorId.current = null;
  };
  const toggleSelected = (id: string) => {
    anchorId.current = id;
    setSelectedIds((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };
  const selectRange = (id: string) => {
    const ids = selectable.map((m) => m.id);
    const from = ids.indexOf(anchorId.current ?? activeMeetingId ?? id);
    const to = ids.indexOf(id);
    if (from === -1) return toggleSelected(id);
    const [a, b] = from < to ? [from, to] : [to, from];
    setSelectedIds(new Set(ids.slice(a, b + 1)));
  };

  const handleCardClick = (meeting: Meeting, e: ClickModifiers) => {
    if (isSidebarCollapsed) return onMeetingSelect(meeting.id);
    if (meeting.status === "recording") {
      if (!isSelecting) onMeetingSelect(meeting.id);
      return;
    }
    if (e.shiftKey) selectRange(meeting.id);
    else if (e.metaKey || isSelecting) toggleSelected(meeting.id);
    else onMeetingSelect(meeting.id);
  };

  const requestDeleteSelected = () =>
    setPendingDelete(selectable.filter((m) => selectedIds.has(m.id)));

  // Keyboard: Delete/Backspace deletes the selection, Escape clears it.
  // Ignored while typing or while a dialog is open.
  useEffect(() => {
    if (!isSelecting) return;
    const onKey = (e: KeyboardEvent) => {
      const target = e.target as HTMLElement | null;
      if (target?.closest("input, textarea, [contenteditable]")) return;
      if (settingsOpen || pendingDelete) return;
      if (e.key === "Escape") clearSelection();
      else if (e.key === "Delete" || e.key === "Backspace") {
        e.preventDefault();
        requestDeleteSelected();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  });

  // The collapsed rail has no room for selection controls.
  useEffect(() => {
    if (isSidebarCollapsed) clearSelection();
  }, [isSidebarCollapsed]);

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
          "flex items-center h-[78px] py-[19px]",
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
            icon={
              isSidebarCollapsed ? (
                <ChevronRight className="w-5 h-5" />
              ) : (
                <ChevronLeft className="w-5 h-5" />
              )
            }
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
          className={cn(
            "ds-capture shrink-0",
            isCapturing && "ds-capture--active",
          )}
          style={{ height: 52 }}
        >
          {isCapturing ? (
            <Square className="w-6 h-6" />
          ) : (
            <Mic className="w-6 h-6" />
          )}
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
              {languageInfo(captureLanguage).label} model not installed —
              download
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
              onClick={(e) => handleCardClick(meeting, e)}
              onToggleSelect={() => toggleSelected(meeting.id)}
              onDelete={() => setPendingDelete([meeting])}
              onRename={(title) => onRenameMeeting(meeting.id, title)}
              isCollapsed={isSidebarCollapsed}
              isTitling={meeting.id === titlingMeetingId}
              isSelecting={isSelecting}
              isSelected={selectedIds.has(meeting.id)}
            />
          ))
        )}
      </div>

      {/* Bulk actions — only while something is selected. */}
      {isSelecting && !isSidebarCollapsed && (
        <div className="mt-auto flex items-center justify-between gap-2 px-1 pt-2 border-t border-[var(--ds-border)]">
          <span className="text-sm text-[var(--ds-text-2)]">
            {selectedIds.size} selected
          </span>
          <div className="flex items-center gap-1">
            <Button
              size="sm"
              variant="danger"
              icon={<Trash2 className="w-3.5 h-3.5" />}
              onClick={requestDeleteSelected}
            >
              Delete
            </Button>
            <IconButton
              icon={<X className="w-4 h-4" />}
              label="Clear selection"
              onClick={clearSelection}
            />
          </div>
        </div>
      )}

      <SettingsModal open={settingsOpen} onClose={closeSettings} />

      {/* Delete confirmation — guards against accidental removal. */}
      <ConfirmDialog
        open={pendingDelete !== null}
        title={
          pendingDelete && pendingDelete.length > 1
            ? `Delete ${pendingDelete.length} meetings?`
            : "Delete meeting?"
        }
        message={
          pendingDelete && pendingDelete.length > 1 ? (
            <>
              {pendingDelete.length} meetings and their transcripts will be
              permanently deleted. This can’t be undone.
            </>
          ) : (
            <>
              “{pendingDelete?.[0]?.title}” and its transcript will be
              permanently deleted. This can’t be undone.
            </>
          )
        }
        confirmLabel="Delete"
        confirmVariant="danger"
        confirmIcon={<Trash2 className="w-4 h-4" />}
        onConfirm={() => {
          if (pendingDelete) onDeleteMeetings(pendingDelete.map((m) => m.id));
          setPendingDelete(null);
          clearSelection();
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

/** Modifier keys of the click (or Enter/Space) that activated a card. */
type ClickModifiers = Pick<React.MouseEvent, "metaKey" | "shiftKey">;

interface MeetingCardProps {
  meeting: Meeting;
  isActive: boolean;
  onClick: (e: ClickModifiers) => void;
  onToggleSelect: () => void;
  onDelete: () => void;
  onRename: (title: string) => void;
  isCollapsed: boolean;
  isTitling?: boolean;
  /** Some meeting is selected: checkboxes stay visible, rename/delete hide. */
  isSelecting?: boolean;
  isSelected?: boolean;
}

const MeetingCard = ({
  meeting,
  isActive,
  onClick,
  onToggleSelect,
  onDelete,
  onRename,
  isCollapsed,
  isTitling,
  isSelecting,
  isSelected,
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
            onClick(e);
          }
        }}
        className={cn(
          "flex items-center justify-center min-h-[40px] p-2 rounded-[var(--ds-radius-sm)] cursor-pointer relative",
          isActive ? "bg-[var(--ds-border)]" : "hover:bg-[var(--ds-surface-2)]",
        )}
        title={meeting.title}
      >
        {meeting.status === "recording" ? (
          <div
            className="animate-pulse bg-[var(--ds-rec)]"
            style={{
              width: 8,
              height: 8,
              minWidth: 8,
              minHeight: 8,
              flex: "0 0 auto",
              borderRadius: "50%",
            }}
          />
        ) : (
          <div
            className="bg-[var(--ds-text)]"
            style={{
              width: 8,
              height: 8,
              minWidth: 8,
              minHeight: 8,
              flex: "0 0 auto",
              borderRadius: "50%",
            }}
          />
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
          onClick(e);
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
            style={{
              borderColor: "var(--ds-border-2)",
              color: "var(--ds-text)",
            }}
          />
        ) : isTitling ? (
          <div className="flex items-center gap-1.5 min-w-0 text-[var(--ds-text-2)]">
            <Loader2 className="w-3.5 h-3.5 animate-spin shrink-0" />
            <span className="ds-meeting__title italic">Generating title…</span>
          </div>
        ) : (
          <div className="flex items-center gap-1.5 min-w-0 flex-1">
            {meeting.status !== "recording" && (
              // Hover-revealed checkbox; stays visible while selecting.
              <button
                type="button"
                role="checkbox"
                aria-checked={Boolean(isSelected)}
                aria-label={`Select ${meeting.title}`}
                onClick={(e) => {
                  e.stopPropagation();
                  onToggleSelect();
                }}
                className={cn(
                  "flex items-center justify-center w-4 h-4 shrink-0 rounded-[4px] border transition-opacity",
                  isSelected
                    ? "bg-[var(--ds-accent)] border-[var(--ds-accent)] text-[var(--ds-surface)]"
                    : "border-[var(--ds-border-2)]",
                  isSelecting
                    ? "opacity-100"
                    : "opacity-0 group-hover:opacity-100 focus-visible:opacity-100",
                )}
              >
                {isSelected && <Check className="w-3 h-3" />}
              </button>
            )}
            <p
              onDoubleClick={isSelecting ? undefined : startEditing}
              className="ds-meeting__title"
            >
              {meeting.title}
            </p>
            {!isSelecting && (
              <button
                onClick={startEditing}
                className="opacity-0 group-hover:opacity-100 p-1 rounded shrink-0 transition-colors hover:bg-[var(--ds-border-2)]"
                title="Rename meeting"
              >
                <Pencil className="w-3.5 h-3.5 text-[var(--ds-text-2)]" />
              </button>
            )}
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
        {meeting.status !== "recording" && !isSelecting && (
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
