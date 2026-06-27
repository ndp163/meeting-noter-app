import type { Meta, StoryObj } from "@storybook/react-vite";
import { SetupCard } from "./SetupCard";
import { Step } from "./Step";
import { ProgressBar } from "./ProgressBar";
import { Button } from "./Button";

const meta = {
  title: "Onboarding/SetupCard",
  component: SetupCard,
  parameters: { layout: "centered" },
} satisfies Meta<typeof SetupCard>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Full: Story = {
  render: () => (
    <SetupCard
      subtitle="A few one-time steps and you're ready to record."
      footer={<Button variant="primary" block style={{ marginTop: 22, height: 44 }}>Continue</Button>}
    >
      <Step index={1} label="Microphone permission" state="done" hint="Granted" />
      <Step index={2} label="System audio capture" state="done" hint="Granted" />
      <Step index={3} label="Downloading speech model" state="active" hint="412 / 640 MB">
        <div style={{ marginTop: 6 }}>
          <ProgressBar value={0.64} />
        </div>
      </Step>
      <Step index={4} label="Connect Claude Code (for summaries)" state="todo" hint="Optional" />
    </SetupCard>
  ),
};
