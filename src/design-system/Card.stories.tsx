import type { Meta, StoryObj } from "@storybook/react-vite";
import { Card } from "./Card";

const meta = {
  title: "Surfaces/Card",
  component: Card,
  decorators: [(S) => <div style={{ width: 360 }}><S /></div>],
} satisfies Meta<typeof Card>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {
  args: { padded: true, children: <div style={{ color: "var(--ds-text)" }}>Panel content lives here.</div> },
};
export const Flat: Story = {
  args: { flat: true, padded: true, children: <div style={{ color: "var(--ds-text-2)" }}>Flat surface, hairline border only.</div> },
};
