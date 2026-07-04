import { Captions } from "lucide-react";
import { PillToggle } from "@/design-system";
import { useBoundStore } from "@/store";

/**
 * Live-caption overlay settings. A single master toggle for now; the overlay
 * floats over the meeting app and shows the transcript in real time while
 * recording. See `src/features/caption/caption-window.tsx`.
 */
export const CaptionsTab = () => {
  const enabled = useBoundStore.use.captionEnabled();
  const setEnabled = useBoundStore.use.setCaptionEnabled();

  return (
    <div className="flex flex-col gap-4">
      <div className="ds-card ds-card--flat flex items-center gap-3 p-4">
        <span
          className="flex items-center justify-center w-9 h-9 rounded-full shrink-0"
          style={{ background: "var(--ds-surface-2)", color: "var(--ds-accent)" }}
        >
          <Captions className="w-4 h-4" />
        </span>
        <div className="flex flex-col min-w-0 flex-1">
          <span className="text-sm font-medium text-[var(--ds-text)]">
            Live captions
          </span>
          <span className="text-xs text-[var(--ds-text-3)]">
            Float a caption strip over the meeting app while recording. Shows the
            latest line per speaker; nothing leaves your machine.
          </span>
        </div>
        <PillToggle
          options={[
            { id: "on", label: "On" },
            { id: "off", label: "Off" },
          ]}
          value={enabled ? "on" : "off"}
          onChange={(id) => setEnabled(id === "on")}
        />
      </div>
    </div>
  );
};
