import { useState } from "react";
import type { Meta, StoryObj } from "@storybook/react-vite";
import { PillToggle } from "./PillToggle";

const meta = {
  title: "Navigation/PillToggle",
  component: PillToggle,
  args: {
    options: [
      { id: "en", label: "Original" },
      { id: "vi", label: "Tiếng Việt" },
    ],
    value: "en",
  },
} satisfies Meta<typeof PillToggle>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Language: Story = {
  render: () => {
    const [v, setV] = useState("en");
    return (
      <PillToggle
        options={[
          { id: "en", label: "Original" },
          { id: "vi", label: "Tiếng Việt" },
        ]}
        value={v}
        onChange={setV}
      />
    );
  },
};
