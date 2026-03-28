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
    code: "S003",
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
    code: "S001",
    severity: "high",
    category: "Auth Gap",
    title: "Missing authorization check on withdraw()",
    location: "src/lib.rs:78",
    suggestion: "Add require_auth() before state mutation.",
    raw: null,
  },
  {
    id: "3",
    code: "S002",
    severity: "medium",
    category: "Panic/Unwrap",
    title: "Using unwrap()",
    location: "src/lib.rs:105",
    snippet: `let value = input.parse::<u64>().unwrap();`,
    line: 1,
    suggestion: "Handle the error with a Result type.",
    raw: null,
  },
  {
    id: "4",
    code: "LEDGER_SIZE_RISK",
    severity: "medium",
    category: "Ledger Size",
    title: "Storage struct approaching size limit",
    location: "src/lib.rs:12",
    snippet: "1800 bytes (limit: 2048)",
    raw: null,
  },
  {
    id: "5",
    code: "UNSAFE_PATTERN",
    severity: "medium",
    category: "Unsafe Pattern",
    title: "Panic",
    location: "src/lib.rs:33",
    snippet: `panic!("unexpected state")`,
    line: 1,
    raw: null,
  },
  {
    id: "6",
    code: "CUSTOM_RULE",
    severity: "low",
    category: "Custom Rule",
    title: "no_floating_point",
    location: "src/lib.rs:50",
    snippet: `let x: f64 = 1.0;`,
    line: 1,
    raw: null,
  },
  {
    id: "7",
    code: "S005",
    severity: "high",
    category: "Storage Collision",
    title: 'Storage key collision on "BALANCE"',
    location: "src/token.rs:22",
    snippet: "Contracts: token_a, token_b",
    raw: null,
  },
  {
    id: "8",
    code: "S008",
    severity: "medium",
    category: "Event Issue",
    title: 'Missing fields in event "Transfer"',
    location: "src/lib.rs:90",
    suggestion: "Include all required fields in the event payload.",
    raw: null,
  },
  {
    id: "9",
    code: "S009",
    severity: "medium",
    category: "Unhandled Result",
    title: "Unhandled result from env.storage().get()",
    location: "src/lib.rs:61",
    snippet: `env.storage().get()`,
    suggestion: "Handle the Result with ? or match.",
    raw: null,
  },
  {
    id: "10",
    code: "S010",
    severity: "high",
    category: "Upgrade Issue",
    title: "Missing migration handler",
    location: "src/upgrade.rs:5",
    snippet: "Contract upgrade does not include a data migration step.",
    suggestion: "Implement a migrate() function for safe upgrades.",
    raw: null,
  },
  {
    id: "11",
    code: "S011",
    severity: "critical",
    category: "SMT Violation",
    title: "Property violation: balance_non_negative",
    location: "src/token.rs:45",
    snippet: "Balance can become negative after unchecked subtraction.",
    suggestion: "Add a pre-condition check before subtraction.",
    raw: null,
  },
  {
    id: "12",
    code: "CVE-2024-1234",
    severity: "critical",
    category: "Vulnerability Match",
    title: "Known reentrancy vulnerability",
    location: "src/lib.rs:120",
    snippet: "Matches known vulnerability pattern CVE-2024-1234.",
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
