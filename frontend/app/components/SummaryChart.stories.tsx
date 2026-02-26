import type { Meta, StoryObj } from "@storybook/react";
import { SummaryChart } from "./SummaryChart";
import type { Finding } from "../types";

const meta: Meta<typeof SummaryChart> = {
  title: "Components/SummaryChart",
  component: SummaryChart,
  tags: ["autodocs"],
  parameters: {
    layout: "padded",
    docs: {
      description: {
        component:
          "Horizontal bar chart showing the distribution of findings across severity levels. Bar widths are relative to the maximum count in any single category.",
      },
    },
  },
};

export default meta;
type Story = StoryObj<typeof SummaryChart>;

function makeFinding(
  severity: "critical" | "high" | "medium" | "low",
  n: number,
): Finding[] {
  return Array.from({ length: n }, (_, i) => ({
    id: `${severity}-${i}`,
    severity,
    category: "test",
    title: `${severity} finding`,
    location: "src/lib.rs",
    raw: null,
  }));
}

export const Balanced: Story = {
  args: {
    findings: [
      ...makeFinding("critical", 2),
      ...makeFinding("high", 4),
      ...makeFinding("medium", 6),
      ...makeFinding("low", 3),
    ],
  },
};

export const CriticalHeavy: Story = {
  args: {
    findings: [...makeFinding("critical", 8), ...makeFinding("high", 1)],
  },
};

export const NoFindings: Story = {
  args: { findings: [] },
};

export const LowOnly: Story = {
  args: { findings: makeFinding("low", 10) },
};
