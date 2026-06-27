import type { StorybookConfig } from "@storybook/react-vite";

const config: StorybookConfig = {
  // Only the design-system stories — keep the app out of the catalogue.
  stories: ["../src/design-system/**/*.stories.@(ts|tsx)"],
  addons: [],
  framework: {
    name: "@storybook/react-vite",
    options: {},
  },
};

export default config;
