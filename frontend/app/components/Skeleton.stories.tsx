import type { Meta, StoryObj } from "@storybook/react";
import {
  Skeleton,
  FindingsListSkeleton,
  ContractCardSkeleton,
} from "./Skeleton";

const meta: Meta<typeof Skeleton> = {
  title: "Components/Skeleton",
  component: Skeleton,
  tags: ["autodocs"],
  parameters: {
    layout: "padded",
    docs: {
      description: {
        component:
          "Pulsing placeholder block used while data loads. The composed skeletons (findings list, contract card, dashboard cards) mirror their real layouts to avoid reflow.",
      },
    },
  },
};

export default meta;
type Story = StoryObj<typeof Skeleton>;

export const Default: Story = {
  args: { width: 240, height: "1rem" },
};

export const Rounded: Story = {
  args: { width: 96, height: 96, rounded: "full" },
};

export const FindingsList: Story = {
  render: () => <FindingsListSkeleton rows={3} />,
};

export const ContractCard: Story = {
  render: () => <ContractCardSkeleton />,
};
