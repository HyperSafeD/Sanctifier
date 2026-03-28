import type { Meta, StoryObj } from "@storybook/react";
import { useLayoutEffect, type ComponentType, type ReactNode } from "react";
import { ThemeToggle } from "./ThemeToggle";
import { ThemeProvider } from "../providers/theme-provider";

type ThemeMode = "light" | "dark";

function applyTheme(theme: ThemeMode) {
  const root = document.documentElement;
  root.dataset.theme = theme;
  root.classList.toggle("dark", theme === "dark");
  root.style.colorScheme = theme;
  window.localStorage.setItem("theme", theme);
}

function ThemeStory({ theme, children }: { theme: ThemeMode; children: ReactNode }) {
  useLayoutEffect(() => {
    const previousTheme = document.documentElement.dataset.theme;
    const previousDark = document.documentElement.classList.contains("dark");

    applyTheme(theme);

    return () => {
      const root = document.documentElement;
      if (previousTheme === undefined) {
        delete root.dataset.theme;
      } else {
        root.dataset.theme = previousTheme;
      }
      root.classList.toggle("dark", previousDark);
      root.style.colorScheme = previousDark ? "dark" : "light";
    };
  }, [theme]);

  return <ThemeProvider>{children}</ThemeProvider>;
}

function LightThemeStory(Story: ComponentType) {
  return (
    <ThemeStory theme="light">
      <Story />
    </ThemeStory>
  );
}

function DarkThemeStory(Story: ComponentType) {
  return (
    <ThemeStory theme="dark">
      <Story />
    </ThemeStory>
  );
}

const meta: Meta<typeof ThemeToggle> = {
  title: "Components/ThemeToggle",
  component: ThemeToggle,
  tags: ["autodocs"],
  parameters: {
    layout: "centered",
    docs: {
      description: {
        component:
          "A button that toggles between light and dark mode through the dashboard theme provider.",
      },
    },
  },
};

export default meta;
type Story = StoryObj<typeof ThemeToggle>;

export const Light: Story = {
  decorators: [LightThemeStory],
};

export const Dark: Story = {
  decorators: [DarkThemeStory],
};
