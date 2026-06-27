import type { Meta, StoryObj } from "@storybook/react-vite";
import { Prose } from "./Prose";

const SUMMARY = `### TL;DR
Mobile design review aligned on the new navigation; two blocking issues remain before handoff. Final assets due Friday.

### Key points
- Bottom-tab navigation approved over the hamburger pattern.
- Color contrast on secondary buttons fails AA — needs a darker tone.
- Empty states still missing for three core screens.

### Action items
- Priya — fix button contrast and re-export tokens (Wed).
- You — draft empty-state copy for review (Thu).
- Alex — finalize handoff package (Fri).
`;

const meta = {
  title: "Content/Prose",
  component: Prose,
  decorators: [(S) => <div style={{ width: 620 }}><S /></div>],
} satisfies Meta<typeof Prose>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Summary: Story = { args: { markdown: SUMMARY } };

export const WithTldrCallout: Story = {
  render: () => (
    <div style={{ width: 620 }}>
      <Prose>
        <h3>TL;DR</h3>
        <div className="ds-tldr">
          <p>Two blocking issues remain before handoff. Final assets due Friday.</p>
        </div>
        <h3>Action items</h3>
        <ul>
          <li className="ds-prose__task">Priya — fix button contrast (Wed).</li>
          <li className="ds-prose__task">Alex — finalize handoff package (Fri).</li>
        </ul>
      </Prose>
    </div>
  ),
};
