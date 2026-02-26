import type { Meta, StoryObj } from "@storybook/react";
import { fn } from "@storybook/test";
import { SeverityFilter } from "./SeverityFilter";

const meta: Meta<typeof SeverityFilter> = {
  title: "Components/SeverityFilter",
  component: SeverityFilter,
  tags: ["autodocs"],
  parameters: {
    layout: "padded",
    docs: {
      description: {
        component:
          "A pill-style filter bar for selecting finding severity levels. Supports 'all' plus the four severity tiers. Active selection is highlighted with the corresponding severity color.",
      },
    },
  },
  argTypes: {
    selected: {
      control: "select",
      options: ["all", "critical", "high", "medium", "low"],
    },
  },
};

export default meta;
type Story = StoryObj<typeof SeverityFilter>;

export const AllSelected: Story = {
  args: {
    selected: "all",
    onChange: fn(),
  },
};

export const CriticalSelected: Story = {
  args: {
    selected: "critical",
    onChange: fn(),
  },
};

export const MediumSelected: Story = {
  args: {
    selected: "medium",
    onChange: fn(),
  },
};
