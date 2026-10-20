import type { Meta, StoryObj } from "@storybook/react";
import { RiskBadge } from "./RiskBadge";
import { RISK_ORDER } from "../lib/contracts-model";

const meta: Meta<typeof RiskBadge> = {
  title: "Components/RiskBadge",
  component: RiskBadge,
  tags: ["autodocs"],
  parameters: {
    layout: "centered",
    docs: {
      description: {
        component:
          "Colored pill showing a contract risk level (critical, high, medium, low, info). Colors adapt to light, dark and high-contrast themes.",
      },
    },
  },
  argTypes: {
    risk: {
      control: "select",
      options: RISK_ORDER,
    },
  },
};

export default meta;
type Story = StoryObj<typeof RiskBadge>;

export const Default: Story = {
  args: { risk: "high" },
};

export const AllLevels: Story = {
  args: { risk: "critical" },
  render: () => (
    <div className="flex flex-wrap gap-2">
      {RISK_ORDER.map((risk) => (
        <RiskBadge key={risk} risk={risk} />
      ))}
    </div>
  ),
};
