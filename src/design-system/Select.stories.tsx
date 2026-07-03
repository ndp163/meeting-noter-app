import { useState } from "react";
import { Check, Download } from "lucide-react";
import type { Meta, StoryObj } from "@storybook/react-vite";
import { Select } from "./Select";

const meta = {
  title: "Inputs/Select",
  component: Select,
  args: {
    label: "Transcribe language",
    value: "en",
    options: [
      { id: "en", label: "English", hint: "English" },
      { id: "ja", label: "日本語", hint: "Japanese" },
    ],
  },
} satisfies Meta<typeof Select>;

export default meta;
type Story = StoryObj<typeof meta>;

export const LanguagePicker: Story = {
  render: () => {
    const [v, setV] = useState("en");
    return (
      <div style={{ width: 300 }}>
        <Select
          label="Transcribe language"
          value={v}
          onChange={setV}
          options={[
            {
              id: "en",
              label: "English",
              hint: "English",
              trailing: <Check className="w-4 h-4 text-[var(--ds-ok)]" />,
            },
            {
              id: "ja",
              label: "日本語",
              hint: "Japanese",
              trailing: (
                <>
                  <Download className="w-3.5 h-3.5" />
                  120 MB
                </>
              ),
            },
          ]}
        />
      </div>
    );
  },
};
