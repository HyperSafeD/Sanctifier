import type { Meta, StoryObj } from "@storybook/react";
import { CallGraph } from "./CallGraph";
import type { CallGraphNode, CallGraphEdge } from "../types";

const meta: Meta<typeof CallGraph> = {
  title: "Components/CallGraph",
  component: CallGraph,
  tags: ["autodocs"],
  parameters: {
    layout: "padded",
    docs: {
      description: {
        component:
          "Renders an SVG-based call graph showing the relationships between contract functions, storage slots, and external calls. Nodes are color-coded by type and edges indicate call, mutation, or read relationships.",
      },
    },
  },
};

export default meta;
type Story = StoryObj<typeof CallGraph>;

const sampleNodes: CallGraphNode[] = [
  { id: "transfer", label: "transfer()", type: "function", severity: "high" },
  { id: "approve", label: "approve()", type: "function" },
  { id: "balance", label: "balances", type: "storage" },
  { id: "allowance", label: "allowances", type: "storage" },
  { id: "oracle", label: "price_oracle", type: "external", severity: "medium" },
];

const sampleEdges: CallGraphEdge[] = [
  { source: "transfer", target: "balance", type: "mutates" },
  { source: "approve", target: "allowance", type: "mutates" },
  { source: "transfer", target: "oracle", type: "calls" },
  { source: "transfer", target: "approve", type: "calls" },
];

export const Default: Story = {
  args: {
    nodes: sampleNodes,
    edges: sampleEdges,
  },
};

export const Empty: Story = {
  args: {
    nodes: [],
    edges: [],
  },
};

export const FunctionsOnly: Story = {
  args: {
    nodes: [
      { id: "mint", label: "mint()", type: "function" },
      { id: "burn", label: "burn()", type: "function" },
    ],
    edges: [{ source: "mint", target: "burn", type: "calls" }],
  },
};

export const WithSeverities: Story = {
  args: {
    nodes: [
      {
        id: "withdraw",
        label: "withdraw()",
        type: "function",
        severity: "critical",
      },
      { id: "deposit", label: "deposit()", type: "function", severity: "low" },
      { id: "vault", label: "vault_storage", type: "storage" },
      {
        id: "external_amm",
        label: "amm_router",
        type: "external",
        severity: "high",
      },
    ],
    edges: [
      { source: "withdraw", target: "vault", type: "mutates" },
      { source: "deposit", target: "vault", type: "mutates" },
      { source: "withdraw", target: "external_amm", type: "calls" },
    ],
  },
};
