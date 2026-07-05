import { useState } from "react";
import type { Meta, StoryObj } from "@storybook/react-vite";
import { Stepper } from "./Stepper";

const meta = {
  title: "Inputs/Stepper",
  component: Stepper,
  args: {
    value: null,
    min: 1,
    max: 20,
    placeholder: "Auto",
    "aria-label": "Number of speakers",
  },
} satisfies Meta<typeof Stepper>;

export default meta;
type Story = StoryObj<typeof meta>;

/** Unset state — shows the placeholder; `+` picks `min`. */
export const Auto: Story = {
  render: (args) => {
    const [v, setV] = useState<number | null>(null);
    return <Stepper {...args} value={v} onChange={setV} />;
  },
};

/** A picked count; stepping below `min` returns to Auto. */
export const WithValue: Story = {
  render: (args) => {
    const [v, setV] = useState<number | null>(3);
    return <Stepper {...args} value={v} onChange={setV} />;
  },
};

export const Disabled: Story = {
  args: { value: 3, disabled: true },
};
