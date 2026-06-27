import { useState } from "react";
import type { Meta, StoryObj } from "@storybook/react-vite";
import { Tabs } from "./Tabs";

const ITEMS = [
  { id: "transcript", label: "Transcript" },
  { id: "summary", label: "Summary" },
  { id: "diarization", label: "Diarization" },
];

const meta = {
  title: "Navigation/Tabs",
  component: Tabs,
  args: { items: ITEMS, value: "transcript" },
} satisfies Meta<typeof Tabs>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {
  render: () => {
    const [value, setValue] = useState("transcript");
    return <Tabs items={ITEMS} value={value} onChange={setValue} />;
  },
};
