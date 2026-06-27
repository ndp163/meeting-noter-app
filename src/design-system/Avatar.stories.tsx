import type { Meta, StoryObj } from "@storybook/react-vite";
import { Avatar } from "./Avatar";

const meta = {
  title: "Data Display/Avatar",
  component: Avatar,
  args: { name: "Alex", size: "md" },
  argTypes: { size: { control: "inline-radio", options: ["sm", "md", "lg"] } },
} satisfies Meta<typeof Avatar>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {};
export const Group: Story = {
  render: () => (
    <div style={{ display: "flex", gap: 8 }}>
      <Avatar name="Alex" />
      <Avatar name="Sarah" />
      <Avatar name="Priya" />
      <Avatar name="You" />
    </div>
  ),
};
