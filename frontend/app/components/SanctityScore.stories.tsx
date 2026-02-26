import type { Meta, StoryObj } from "@storybook/react";
import { SanctityScore } from "./SanctityScore";
import type { Finding } from "../types";

const meta: Meta<typeof SanctityScore> = {
  title: "Components/SanctityScore",
  component: SanctityScore,
  tags: ["autodocs"],
  parameters: {
    layout: "centered",
    docs: {
      description: {
        component:
          "Displays the overall security score as a semi-circular gauge. Score is calculated from findings using severity-based weights (critical: 15, high: 10, medium: 5, low: 2). Includes a letter grade (A–F) and a risk summary.",
      },
    },
  },
};

export default meta;
type Story = StoryObj<typeof SanctityScore>;

function makeFinding(
  severity: "critical" | "high" | "medium" | "low",
  n: number,
): Finding[] {
  return Array.from({ length: n }, (_, i) => ({
    id: `${severity}-${i}`,
    severity,
    category: "test",
    title: `${severity} finding ${i + 1}`,
    location: "src/lib.rs",
    raw: null,
  }));
}

export const PerfectScore: Story = {
  args: { findings: [] },
};

export const GradeA: Story = {
  args: { findings: makeFinding("low", 3) },
};

export const GradeB: Story = {
  args: { findings: [...makeFinding("medium", 2), ...makeFinding("low", 3)] },
};

export const GradeC: Story = {
  args: { findings: [...makeFinding("high", 2), ...makeFinding("medium", 1)] },
};

export const GradeF: Story = {
  args: {
    findings: [
      ...makeFinding("critical", 3),
      ...makeFinding("high", 2),
      ...makeFinding("medium", 2),
    ],
  },
};
