import { Modal } from "@/design-system";
import { ModelManager } from "@/features/models/model-manager";

interface SettingsModalProps {
  open: boolean;
  onClose: () => void;
}

/** Settings overlay. Currently hosts the language/model manager. */
export const SettingsModal = ({ open, onClose }: SettingsModalProps) => (
  <Modal open={open} title="Languages & models" onClose={onClose}>
    <ModelManager />
  </Modal>
);
