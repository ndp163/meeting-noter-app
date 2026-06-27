import type { Meta, StoryObj } from "@storybook/react-vite";
import { SpeakerSegment } from "./SpeakerSegment";

const meta = {
  title: "Data Display/SpeakerSegment",
  component: SpeakerSegment,
  args: {
    speaker: "Alex",
    timestamp: "16:45:02",
    content: "How are you feeling about the workload this sprint?",
  },
  decorators: [(S) => <div style={{ width: 560 }}><S /></div>],
} satisfies Meta<typeof SpeakerSegment>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {};
export const Renameable: Story = { args: { onRename: () => {} } };

export const Conversation: Story = {
  render: () => (
    <div style={{ width: 560 }}>
      <SpeakerSegment speaker="Alex" timestamp="16:45:02" divided onRename={() => {}} content="How are you feeling about the workload this sprint?" />
      <SpeakerSegment speaker="You" timestamp="16:45:09" divided onRename={() => {}} content="Manageable. The crash fix freed up time I'd budgeted for support." />
      <SpeakerSegment speaker="Sarah" timestamp="16:45:30" content="Jumping in late — can we also scope the onboarding download progress?" />
    </div>
  ),
};
