import { Captions, Globe, RefreshCw, Sparkles, Languages } from "lucide-react";
import type { LucideIcon } from "lucide-react";
import { Modal } from "@/design-system";
import { ModelManager } from "@/features/models/model-manager";
import { SummaryTab } from "./summary-tab";
import { TranslateTab } from "./translate-tab";
import { CaptionsTab } from "./captions-tab";
import { UpdatesTab } from "./updates-tab";
import { cn } from "@/lib/utils";
import { useBoundStore } from "@/store";
import type { SettingsTab } from "@/store/ui.slice";

interface SettingsModalProps {
  open: boolean;
  onClose: () => void;
}

interface NavItem {
  id: SettingsTab;
  label: string;
  icon: LucideIcon;
}

const NAV: NavItem[] = [
  { id: "models", label: "Languages", icon: Globe },
  { id: "summary", label: "AI summary", icon: Sparkles },
  { id: "translate", label: "Translation", icon: Languages },
  { id: "captions", label: "Captions", icon: Captions },
  { id: "updates", label: "Updates", icon: RefreshCw },
];

/**
 * Settings overlay. A vertical section rail on the left keeps labels on one line
 * and scales as sections are added; the active section renders on the right.
 */
export const SettingsModal = ({ open, onClose }: SettingsModalProps) => {
  const tab = useBoundStore.use.settingsTab();
  const setTab = useBoundStore.use.setSettingsTab();

  const active = NAV.find((n) => n.id === tab) ?? NAV[0];

  return (
    <Modal
      open={open}
      title="Settings"
      onClose={onClose}
      className="w-[min(680px,calc(100vw_-_48px))]! max-w-[680px]!"
    >
      <div className="flex gap-4 w-full min-h-[360px]">
        {/* Section rail */}
        <nav
          className="flex w-[132px] shrink-0 flex-col gap-0.5"
          aria-label="Settings sections"
        >
          {NAV.map((item) => {
            const Icon = item.icon;
            const selected = item.id === tab;
            return (
              <button
                key={item.id}
                type="button"
                aria-current={selected ? "page" : undefined}
                onClick={() => setTab(item.id)}
                className={cn(
                  "flex items-center gap-2.5 rounded-[var(--ds-radius-sm)] px-3 py-2 text-sm text-left transition-colors cursor-pointer whitespace-nowrap",
                  selected
                    ? "bg-[var(--ds-surface-2)] text-[var(--ds-text)] font-medium"
                    : "text-[var(--ds-text-2)] hover:bg-[var(--ds-surface-2)] hover:text-[var(--ds-text)]",
                )}
              >
                <Icon className="w-4 h-4 shrink-0" />
                {item.label}
              </button>
            );
          })}
        </nav>

        {/* Active section */}
        <div className="flex-1 min-w-0 border-l border-[var(--ds-border)] pl-5">
          <h3 className="mb-4 text-sm font-semibold text-[var(--ds-text)]">
            {active.label}
          </h3>
          {tab === "models" ? (
            <ModelManager />
          ) : tab === "summary" ? (
            <SummaryTab />
          ) : tab === "translate" ? (
            <TranslateTab />
          ) : tab === "captions" ? (
            <CaptionsTab />
          ) : (
            <UpdatesTab />
          )}
        </div>
      </div>
    </Modal>
  );
};
