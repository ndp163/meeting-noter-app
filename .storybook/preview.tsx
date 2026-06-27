import type { Preview } from "@storybook/react-vite";
import "../src/design-system/styles.css";

const preview: Preview = {
  parameters: {
    layout: "centered",
    backgrounds: {
      default: "canvas",
      values: [
        { name: "canvas", value: "#fbfbfa" },
        { name: "surface", value: "#ffffff" },
      ],
    },
    controls: { matchers: { color: /(background|color)$/i, date: /Date$/i } },
  },
  decorators: [
    (Story) => (
      <div className="ds-root" style={{ padding: 24 }}>
        <Story />
      </div>
    ),
  ],
};

export default preview;
