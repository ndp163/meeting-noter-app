import { useState } from "react";
import { Trash2 } from "lucide-react";
import type { Meta, StoryObj } from "@storybook/react-vite";
import { ConfirmDialog } from "./ConfirmDialog";
import { Button } from "./Button";

const meta = {
  title: "Feedback/ConfirmDialog",
  component: ConfirmDialog,
  args: {
    open: false,
    title: "Delete meeting?",
    message: "This can’t be undone.",
    onConfirm: () => {},
    onCancel: () => {},
  },
} satisfies Meta<typeof ConfirmDialog>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Destructive: Story = {
  render: () => {
    const [open, setOpen] = useState(false);
    return (
      <>
        <Button variant="danger" onClick={() => setOpen(true)}>
          Delete meeting
        </Button>
        <ConfirmDialog
          open={open}
          title="Delete meeting?"
          message="This meeting and its transcript will be permanently deleted. This can’t be undone."
          confirmLabel="Delete"
          confirmVariant="danger"
          confirmIcon={<Trash2 className="w-4 h-4" />}
          onConfirm={() => setOpen(false)}
          onCancel={() => setOpen(false)}
        />
      </>
    );
  },
};
