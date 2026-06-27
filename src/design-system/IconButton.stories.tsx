import type { Meta, StoryObj } from "@storybook/react-vite";
import { ChevronLeft } from "lucide-react";
import { IconButton } from "./IconButton";

const meta = {
  title: "Controls/IconButton",
  component: IconButton,
  args: { icon: <ChevronLeft size={18} />, label: "Collapse sidebar" },
} satisfies Meta<typeof IconButton>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {};
