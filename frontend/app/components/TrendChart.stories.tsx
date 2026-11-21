import type { Meta, StoryObj } from "@storybook/react";
import { fn } from "@storybook/test";
import { TrendChart } from "./TrendChart";
import type { ScanRecord } from "../lib/scan-history";

const DAY_MS = 24 * 60 * 60 * 1000;
const now = Date.now();

const records: ScanRecord[] = [0, 1, 2, 3, 4, 5, 6].map((day) => ({
  workspace: "token_contract",
  timestamp: now - (6 - day) * DAY_MS,
  critical: 0,
  high: day % 3,
  medium: day,
  low: 2,
  total: day % 3 + day + 2,
}));

const meta: Meta<typeof TrendChart> = {
  title: "Components/TrendChart",
  component: TrendChart,
  tags: ["autodocs"],
  parameters: {
    layout: "padded",
    docs: {
      description: {
        component:
          "Sparkline of finding counts for a single severity over time. Rendered once per severity inside TrendPanel.",
      },
    },
  },
  args: {
    onClear: fn(),
  },
};

export default meta;
type Story = StoryObj<typeof TrendChart>;

export const High: Story = {
  args: { records, severity: "high", color: "#ea580c" },
};

export const Empty: Story = {
  args: { records: [], severity: "high", color: "#ea580c" },
};
