import { useEffect, useState } from "react";
import { FolderOpen, HardDrive, RotateCcw } from "lucide-react";
import { Button, ConfirmDialog, Select } from "@/design-system";
import { useBoundStore } from "@/store";
import {
  getRecordingsDir,
  getRetentionDays,
  openRecordingsDir,
  pickFolder,
  setRecordingsDir,
  setRetentionDays,
  type RecordingsDirInfo,
} from "@/services/meetings";

const RETENTION_OPTIONS = [
  { id: "0", label: "Forever" },
  { id: "7", label: "7 days" },
  { id: "30", label: "30 days" },
  { id: "90", label: "90 days" },
  { id: "180", label: "180 days" },
];

/**
 * Meeting lifecycle. Meetings (transcript + audio) older than the chosen age
 * are deleted automatically each time the app starts, keeping disk use in check.
 * Also where recordings live: the folder can be opened or moved.
 */
export const StorageTab = () => {
  const [days, setDays] = useState<number>();
  const [dir, setDir] = useState<RecordingsDirInfo>();
  // Folder awaiting move confirmation; `null` = reset to default.
  const [pendingDir, setPendingDir] = useState<string | null>();
  const [isMoving, setIsMoving] = useState(false);
  const isRecording = useBoundStore.use.isRecording();
  const meetingCount = useBoundStore.use.meetings().length;
  const bumpStorageVersion = useBoundStore.use.bumpStorageVersion();
  const pushToast = useBoundStore.use.pushToast();

  useEffect(() => {
    getRetentionDays()
      .then(setDays)
      .catch(() => setDays(0));
    void getRecordingsDir().then(setDir);
  }, []);

  const chooseFolder = async () => {
    const picked = await pickFolder();
    if (picked) setPendingDir(picked);
  };

  const moveRecordings = async (target: string | null) => {
    setPendingDir(undefined);
    setIsMoving(true);
    try {
      await setRecordingsDir(target);
      setDir(await getRecordingsDir());
      bumpStorageVersion();
      pushToast("Recordings folder changed.", "success");
    } catch (e) {
      pushToast(String(e), "error");
    } finally {
      setIsMoving(false);
    }
  };

  const handleChange = (id: string) => {
    const next = Number(id);
    setDays(next);
    void setRetentionDays(next);
  };

  return (
    <div className="flex flex-col gap-4">
      <div className="ds-card ds-card--flat flex items-center gap-3 p-4">
        <span
          className="flex items-center justify-center w-9 h-9 rounded-full shrink-0"
          style={{ background: "var(--ds-surface-2)", color: "var(--ds-accent)" }}
        >
          <HardDrive className="w-4 h-4" />
        </span>
        <div className="flex flex-col min-w-0 flex-1">
          <span className="text-sm font-medium text-[var(--ds-text)]">
            Keep meetings for
          </span>
          <span className="text-xs text-[var(--ds-text-3)]">
            Older meetings and their recordings are permanently deleted when the
            app starts.
          </span>
        </div>
        <Select
          className="w-[132px] shrink-0"
          options={RETENTION_OPTIONS}
          value={days === undefined ? "" : String(days)}
          disabled={days === undefined}
          onChange={handleChange}
        />
      </div>

      <div className="ds-card ds-card--flat flex flex-col gap-3 p-4">
        <div className="flex items-center gap-3">
          <span
            className="flex items-center justify-center w-9 h-9 rounded-full shrink-0"
            style={{ background: "var(--ds-surface-2)", color: "var(--ds-accent)" }}
          >
            <FolderOpen className="w-4 h-4" />
          </span>
          <div className="flex flex-col min-w-0 flex-1">
            <span className="text-sm font-medium text-[var(--ds-text)]">
              Recordings folder
            </span>
            <span
              className="text-xs text-[var(--ds-text-3)] truncate"
              title={dir?.path}
            >
              {dir?.path ?? "…"}
            </span>
          </div>
          <Button
            size="sm"
            variant="ghost"
            disabled={!dir}
            onClick={() =>
              openRecordingsDir().catch((e) => pushToast(String(e), "error"))
            }
          >
            Open
          </Button>
        </div>
        <div
          className="flex items-center gap-2 pl-12"
          title={isRecording ? "Stop recording to change the folder" : undefined}
        >
          <Button
            size="sm"
            disabled={!dir || isRecording || isMoving}
            onClick={() => void chooseFolder()}
          >
            {isMoving ? "Moving…" : "Change…"}
          </Button>
          {dir && !dir.isDefault && (
            <Button
              size="sm"
              variant="ghost"
              icon={<RotateCcw className="w-3.5 h-3.5" />}
              disabled={isRecording || isMoving}
              onClick={() => setPendingDir(null)}
            >
              Reset to default
            </Button>
          )}
        </div>
      </div>

      <ConfirmDialog
        open={pendingDir !== undefined}
        title="Move recordings?"
        message={
          <>
            {meetingCount === 1 ? "1 meeting" : `${meetingCount} meetings`}{" "}
            (transcripts and audio) will be moved to{" "}
            <strong>{pendingDir ?? "the default folder"}</strong>.
          </>
        }
        confirmLabel="Move"
        onConfirm={() => void moveRecordings(pendingDir ?? null)}
        onCancel={() => setPendingDir(undefined)}
      />
    </div>
  );
};
