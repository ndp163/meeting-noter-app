import type { Meta, StoryObj } from "@storybook/react-vite";
import { StatusDot } from "./StatusDot";

const meta = {
  title: "Feedback/StatusDot",
  component: StatusDot,
} satisfies Meta<typeof StatusDot>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Listening: Story = { args: { tone: "rec", pulse: true, label: "Listening" } };
export const Idle: Story = { args: { tone: "idle", label: "Idle" } };
export const Preparing: Story = { args: { tone: "idle", pulse: true, label: "Preparing model…" } };

export const AllTones: Story = {
  render: () => (
    <div style={{ display: "flex", flexDirection: "column", gap: 10 }}>
      <StatusDot tone="rec" pulse label="Listening" />
      <StatusDot tone="accent" label="Active" />
      <StatusDot tone="ok" label="Ready" />
      <StatusDot tone="idle" label="Idle" />
    </div>
  ),
};
