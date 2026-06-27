import type { Meta, StoryObj } from "@storybook/react-vite";
import { CaptureButton } from "./CaptureButton";

const meta = {
  title: "Controls/CaptureButton",
  component: CaptureButton,
  decorators: [(S) => <div style={{ width: 276 }}><S /></div>],
} satisfies Meta<typeof CaptureButton>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Idle: Story = { args: { capturing: false } };
export const Recording: Story = { args: { capturing: true } };
