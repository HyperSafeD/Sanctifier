import type { Meta, StoryObj } from "@storybook/react";
import { CodeSnippet } from "./CodeSnippet";

const meta: Meta<typeof CodeSnippet> = {
  title: "Components/CodeSnippet",
  component: CodeSnippet,
  tags: ["autodocs"],
  parameters: {
    layout: "padded",
    docs: {
      description: {
        component:
          "Displays a code block with line numbers and optional line highlighting. Used within FindingsList to show vulnerable code sections.",
      },
    },
  },
};

export default meta;
type Story = StoryObj<typeof CodeSnippet>;

const rustSample = `pub fn transfer(env: Env, from: Address, to: Address, amount: i128) {
    let balance = get_balance(&env, &from);
    if balance < amount {
        panic!("insufficient balance");
    }
    set_balance(&env, &from, balance - amount);
    set_balance(&env, &to, get_balance(&env, &to) + amount);
}`;

export const Default: Story = {
  args: {
    code: rustSample,
  },
};

export const WithHighlightedLine: Story = {
  args: {
    code: rustSample,
    highlightLine: 4,
  },
};

export const SingleLine: Story = {
  args: {
    code: `let x: u64 = a * b;`,
    highlightLine: 1,
  },
};

export const EmptyCode: Story = {
  args: {
    code: "",
  },
};
