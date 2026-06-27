import type { Meta, StoryObj } from "@storybook/react-vite";
import { Plus } from "lucide-react";
import { Button } from "./Button";

const meta = {
  title: "Controls/Button",
  component: Button,
  args: { children: "Generate summary", variant: "ghost", size: "md" },
  argTypes: {
    variant: { control: "inline-radio", options: ["primary", "ghost", "accent"] },
    size: { control: "inline-radio", options: ["sm", "md", "lg"] },
  },
} satisfies Meta<typeof Button>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Primary: Story = { args: { variant: "primary", children: "Continue" } };
export const Ghost: Story = { args: { variant: "ghost", children: "Regenerate" } };
export const Accent: Story = { args: { variant: "accent", children: "Connect Claude" } };
export const Pill: Story = { args: { variant: "ghost", pill: true, children: "Try again" } };
export const WithIcon: Story = {
  args: { variant: "primary", icon: <Plus size={16} />, children: "New meeting" },
};

export const AllVariants: Story = {
  render: () => (
    <div style={{ display: "flex", gap: 12, alignItems: "center" }}>
      <Button variant="primary">Primary</Button>
      <Button variant="ghost">Ghost</Button>
      <Button variant="accent">Accent</Button>
      <Button variant="ghost" pill>
        Pill
      </Button>
    </div>
  ),
};
