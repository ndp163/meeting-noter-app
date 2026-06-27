import type { Meta, StoryObj } from "@storybook/react-vite";
import { AudioPlayer } from "./AudioPlayer";

const meta = {
  title: "Media/AudioPlayer",
  component: AudioPlayer,
  args: { current: "00:31", duration: "02:14", progress: 0.23, playing: false },
  argTypes: { progress: { control: { type: "range", min: 0, max: 1, step: 0.01 } } },
  decorators: [(S) => <div style={{ width: 620 }}><S /></div>],
} satisfies Meta<typeof AudioPlayer>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Paused: Story = {};
export const Playing: Story = { args: { playing: true, progress: 0.5 } };
