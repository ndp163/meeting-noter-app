import { Modal, Tabs } from "@/design-system";
import { ModelManager } from "@/features/models/model-manager";
import { SummaryTab } from "./summary-tab";
import { TranslateTab } from "./translate-tab";
import { UpdatesTab } from "./updates-tab";
import { useBoundStore } from "@/store";
import type { SettingsTab } from "@/store/ui.slice";

interface SettingsModalProps {
  open: boolean;
  onClose: () => void;
}

const TABS = [
  { id: "models", label: "Languages" },
  { id: "summary", label: "AI summary" },
  { id: "translate", label: "Translation" },
  { id: "updates", label: "Updates" },
];

/** Settings overlay. Tabs between languages, the AI summary engine, and updates. */
export const SettingsModal = ({ open, onClose }: SettingsModalProps) => {
  const tab = useBoundStore.use.settingsTab();
  const setTab = useBoundStore.use.setSettingsTab();

  return (
    <Modal open={open} title="Settings" onClose={onClose}>
      <div className="flex flex-col gap-4">
        <Tabs
          className="self-start"
          items={TABS}
          value={tab}
          onChange={(id) => setTab(id as SettingsTab)}
        />
        {tab === "models" ? (
          <ModelManager />
        ) : tab === "summary" ? (
          <SummaryTab />
        ) : tab === "translate" ? (
          <TranslateTab />
        ) : (
          <UpdatesTab />
        )}
      </div>
    </Modal>
  );
};
