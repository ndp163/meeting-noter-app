import type { Meta, StoryObj } from "@storybook/react-vite";
import { Message } from "./Message";

const meta = {
  title: "Data Display/Message",
  component: Message,
  args: {
    speaker: "Sarah",
    timestamp: "00:24",
    content:
      "Great. What's the status on diarization quality? Last I heard speaker labels were drifting on longer calls.",
  },
  decorators: [
    (S) => (
      <div style={{ width: 520 }}>
        <S />
      </div>
    ),
  ],
} satisfies Meta<typeof Message>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Other: Story = {};
export const LocalUser: Story = {
  args: {
    speaker: "You",
    isUser: true,
    content: "Improved. We're clustering on embeddings now.",
  },
};

/** Live ASR: committed text is solid, the interim tail is dimmed + italic. */
export const Interim: Story = {
  args: {
    speaker: "You",
    isUser: true,
    content: "The transcription pipeline is shipped and",
    partial: "stable, and we're now working on",
  },
};

export const Thread: Story = {
  render: () => (
    <div style={{ width: 520 }}>
      <Message
        speaker="Sarah"
        timestamp="00:02"
        divided
        content="Thanks everyone for joining. Let's start with the Q3 roadmap."
      />
      <Message
        speaker="You"
        isUser
        timestamp="00:11"
        divided
        onSeek={() => {}}
        content="Sounds good. The transcription pipeline is shipped and stable."
      />
      <Message
        speaker="Sarah"
        timestamp="00:24"
        content="What's the status on diarization quality?"
      />
    </div>
  ),
};

/** Word-level seek: each word is clickable; the playhead word is highlighted. */
export const WordSeek: Story = {
  args: {
    speaker: "You",
    isUser: true,
    content: "The transcription pipeline is shipped",
    words: [
      { text: "The", start: 24.0, end: 24.2 },
      { text: "transcription", start: 24.2, end: 24.9 },
      { text: "pipeline", start: 24.9, end: 25.4 },
      { text: "is", start: 25.4, end: 25.55 },
      { text: "shipped", start: 25.55, end: 26.0 },
    ],
    activeWordIndex: 2,
    onWordClick: () => {},
  },
};
