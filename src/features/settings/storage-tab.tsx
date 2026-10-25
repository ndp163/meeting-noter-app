import { useEffect, useState } from "react";
import { HardDrive } from "lucide-react";
import { Select } from "@/design-system";
import { getRetentionDays, setRetentionDays } from "@/services/meetings";

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
 */
export const StorageTab = () => {
  const [days, setDays] = useState<number>();

  useEffect(() => {
    getRetentionDays()
      .then(setDays)
      .catch(() => setDays(0));
  }, []);

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
    </div>
  );
};
