import type { Meta, StoryObj } from "@storybook/react-vite";
import { Step } from "./Step";
import { ProgressBar } from "./ProgressBar";

const meta = {
  title: "Onboarding/Step",
  component: Step,
  args: { index: 3, label: "Downloading speech model", state: "active" },
  argTypes: { state: { control: "inline-radio", options: ["todo", "active", "done"] } },
  decorators: [(S) => <div style={{ width: 460 }}><S /></div>],
} satisfies Meta<typeof Step>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Todo: Story = { args: { state: "todo", index: 4, label: "Connect Claude Code", hint: "Optional" } };
export const Active: Story = { args: { hint: "412 / 640 MB" } };
export const Done: Story = { args: { state: "done", index: 1, label: "Microphone permission", hint: "Granted" } };

export const WithProgress: Story = {
  render: () => (
    <div style={{ width: 460 }}>
      <Step index={3} label="Downloading speech model" state="active" hint="412 / 640 MB">
        <div style={{ marginTop: 6 }}>
          <ProgressBar value={0.64} />
        </div>
      </Step>
    </div>
  ),
};
