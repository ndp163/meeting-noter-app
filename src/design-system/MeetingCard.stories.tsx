import type { Meta, StoryObj } from "@storybook/react-vite";
import { MeetingCard } from "./MeetingCard";

const meta = {
  title: "Data Display/MeetingCard",
  component: MeetingCard,
  args: {
    title: "Q3 Roadmap Sync",
    timestamp: "10:24 · 27/06/26",
    source: "Teams",
  },
  decorators: [(S) => <div style={{ width: 276 }}><S /></div>],
} satisfies Meta<typeof MeetingCard>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {};
export const Active: Story = { args: { active: true } };
export const Recording: Story = { args: { active: true, recording: true } };

export const List: Story = {
  render: () => (
    <div style={{ width: 276, display: "flex", flexDirection: "column", gap: 4 }}>
      <MeetingCard title="Q3 Roadmap Sync" timestamp="10:24 · 27/06/26" source="Teams" active recording />
      <MeetingCard title="Design Review — Mobile" timestamp="09:10 · 27/06/26" source="Meet" />
      <MeetingCard title="1:1 with Alex" timestamp="16:45 · 26/06/26" source="Zoom" />
    </div>
  ),
};
