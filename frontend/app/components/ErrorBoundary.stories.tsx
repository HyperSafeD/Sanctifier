import type { Meta, StoryObj } from "@storybook/react";
import { ErrorBoundary } from "./ErrorBoundary";

const meta: Meta<typeof ErrorBoundary> = {
  title: "Components/ErrorBoundary",
  component: ErrorBoundary,
  tags: ["autodocs"],
  parameters: {
    layout: "padded",
    docs: {
      description: {
        component:
          "Catches rendering errors in child components and displays a fallback UI with a retry button instead of crashing the entire page.",
      },
    },
  },
};

export default meta;
type Story = StoryObj<typeof ErrorBoundary>;

function ThrowingChild() {
  throw new Error("Test error: component crashed unexpectedly");
  return null;
}

function HealthyChild() {
  return (
    <div className="rounded-lg border border-green-300 bg-green-50 p-6 text-center text-green-700">
      <p className="font-medium">Component rendered successfully</p>
    </div>
  );
}

export const ErrorState: Story = {
  render: () => (
    <ErrorBoundary>
      <ThrowingChild />
    </ErrorBoundary>
  ),
};

export const HealthyState: Story = {
  render: () => (
    <ErrorBoundary>
      <HealthyChild />
    </ErrorBoundary>
  ),
};

export const CustomFallback: Story = {
  render: () => (
    <ErrorBoundary
      fallback={
        <div className="rounded-lg border border-amber-300 bg-amber-50 p-6 text-center text-amber-700">
          <p className="font-medium">Custom fallback UI</p>
        </div>
      }
    >
      <ThrowingChild />
    </ErrorBoundary>
  ),
};
