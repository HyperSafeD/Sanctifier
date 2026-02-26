import type { Meta, StoryObj } from "@storybook/react";
import { FindingsList } from "./FindingsList";
import type { Finding } from "../types";

const meta: Meta<typeof FindingsList> = {
  title: "Components/FindingsList",
  component: FindingsList,
  tags: ["autodocs"],
  parameters: {
    layout: "padded",
    docs: {
      description: {
        component:
          "Renders a filterable list of security findings. Each finding is color-coded by severity and may include an inline code snippet with highlighted vulnerable lines.",
      },
    },
  },
  argTypes: {
    severityFilter: {
      control: "select",
      options: ["all", "critical", "high", "medium", "low"],
    },
  },
};

export default meta;
type Story = StoryObj<typeof FindingsList>;

const sampleFindings: Finding[] = [
  {
    id: "1",
    severity: "critical",
    category: "Arithmetic",
    title: "Unchecked multiplication may overflow",
    location: "src/lib.rs:42",
    snippet: `let total = price * quantity;`,
    line: 1,
    suggestion: "Use checked_mul() or saturating_mul() instead.",
    raw: null,
  },
  {
    id: "2",
    severity: "high",
    category: "Auth Gap",
    title: "Missing authorization check on withdraw()",
    location: "src/lib.rs:78",
    suggestion: "Add require_auth() before state mutation.",
    raw: null,
  },
  {
    id: "3",
    severity: "medium",
    category: "Panic",
    title: "Unwrap on user-supplied input",
    location: "src/lib.rs:105",
    snippet: `let value = input.parse::<u64>().unwrap();`,
    line: 1,
    suggestion: "Handle the error with a Result type.",
    raw: null,
  },
  {
    id: "4",
    severity: "low",
    category: "Ledger Size",
    title: "Storage struct approaching size limit",
    location: "src/lib.rs:12",
    raw: null,
  },
];

export const AllFindings: Story = {
  args: {
    findings: sampleFindings,
    severityFilter: "all",
  },
};

export const CriticalOnly: Story = {
  args: {
    findings: sampleFindings,
    severityFilter: "critical",
  },
};

export const NoResults: Story = {
  args: {
    findings: sampleFindings,
    severityFilter: "low",
  },
};

export const EmptyFindings: Story = {
  args: {
    findings: [],
    severityFilter: "all",
  },
};
