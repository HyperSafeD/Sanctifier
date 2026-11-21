import type { Meta, StoryObj } from "@storybook/react";
import { AnalysisTerminal } from "./AnalysisTerminal";

const meta: Meta<typeof AnalysisTerminal> = {
  title: "Components/AnalysisTerminal",
  component: AnalysisTerminal,
  tags: ["autodocs"],
  parameters: {
    layout: "padded",
    docs: {
      description: {
        component:
          "Scrolling log view showing analysis progress. Shows a running indicator while `isAnalyzing` is true.",
      },
    },
  },
};

export default meta;
type Story = StoryObj<typeof AnalysisTerminal>;

const sampleLogs = [
  "Starting analysis of token_contract.rs",
  "Parsing Soroban contract entry points",
  "Running authorization checks",
  "Warning: unchecked arithmetic in transfer()",
  "Analysis complete: 3 findings",
];

export const Analyzing: Story = {
  args: { logs: sampleLogs, isAnalyzing: true },
};

export const Complete: Story = {
  args: { logs: sampleLogs, isAnalyzing: false },
};

export const Empty: Story = {
  args: { logs: [], isAnalyzing: false },
};
