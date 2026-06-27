import type { Meta, StoryObj } from "@storybook/react-vite";
import { useState } from "react";
import { Modal } from "./Modal";
import { Button } from "./Button";

const meta = {
  title: "Overlays/Modal",
  component: Modal,
} satisfies Meta<typeof Modal>;

export default meta;
type Story = StoryObj<typeof meta>;

const Demo = () => {
  const [open, setOpen] = useState(false);
  return (
    <>
      <Button variant="primary" onClick={() => setOpen(true)}>
        Open modal
      </Button>
      <Modal open={open} title="Settings" onClose={() => setOpen(false)}>
        <p style={{ color: "var(--ds-text-2)", fontSize: 14 }}>
          Dialog body content. Press Escape or click the scrim to close.
        </p>
      </Modal>
    </>
  );
};

export const Default: Story = {
  args: { open: false, onClose: () => {} },
  render: () => <Demo />,
};
