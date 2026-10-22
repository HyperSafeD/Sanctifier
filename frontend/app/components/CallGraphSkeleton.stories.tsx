import type { Meta, StoryObj } from "@storybook/react";
import { CallGraphSkeleton } from "./CallGraphSkeleton";

const meta: Meta<typeof CallGraphSkeleton> = {
  title: "Components/CallGraphSkeleton",
  component: CallGraphSkeleton,
  tags: ["autodocs"],
  parameters: {
    layout: "padded",
    docs: {
      description: {
        component:
          "Loading placeholder for CallGraph. Shown as the dynamic-import fallback so the call-graph layout does not shift when the real graph mounts.",
      },
    },
  },
};

export default meta;
type Story = StoryObj<typeof CallGraphSkeleton>;

export const Default: Story = {};
