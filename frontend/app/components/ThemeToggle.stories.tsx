import type { Meta, StoryObj } from "@storybook/react";
import { ThemeToggle } from "./ThemeToggle";

const meta: Meta<typeof ThemeToggle> = {
  title: "Components/ThemeToggle",
  component: ThemeToggle,
  tags: ["autodocs"],
  parameters: {
    layout: "centered",
    docs: {
      description: {
        component:
          "A button that toggles between light and dark mode by adding or removing the 'dark' class on the document root. Respects the user's system preference on initial load.",
      },
    },
  },
};

export default meta;
type Story = StoryObj<typeof ThemeToggle>;

export const Default: Story = {};
