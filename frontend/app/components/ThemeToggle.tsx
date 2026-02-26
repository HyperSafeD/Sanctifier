"use client";

import { useTheme } from "../providers/theme-provider";

export function ThemeToggle() {
  const { theme, toggleTheme } = useTheme();

  return (
    <button
      onClick={toggleTheme}
      className="rounded-lg border border-zinc-300 dark:border-zinc-600 px-3 py-2 text-sm hover:bg-zinc-100 dark:hover:bg-zinc-800"
      aria-label="Toggle theme"
    >
      {theme === "dark" ? "Switch to Light" : "Switch to Dark"}
    </button>
  );
}
