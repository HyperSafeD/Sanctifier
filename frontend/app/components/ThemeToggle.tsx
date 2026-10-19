"use client";

import { useTheme } from "../providers/theme-provider";
import type { Theme } from "../providers/theme-provider";

const OPTIONS: { value: Theme; label: string }[] = [
  { value: "light",         label: "Light" },
  { value: "dark",          label: "Dark" },
  { value: "system",        label: "System" },
  { value: "high-contrast", label: "High Contrast" },
];

export function ThemeToggle() {
  const { theme, setTheme } = useTheme();

  return (
    <div role="group" aria-label="Theme selector" className="flex rounded-lg border border-zinc-300 dark:border-zinc-600 overflow-hidden text-sm">
      {OPTIONS.map(({ value, label }) => (
        <button
          key={value}
          onClick={() => setTheme(value)}
          aria-pressed={theme === value}
          className={[
            "px-3 py-2 focus:outline-none focus-visible:ring-2 focus-visible:ring-zinc-400 focus-visible:ring-offset-1 transition-colors",
            theme === value
              ? "bg-zinc-900 text-white dark:bg-zinc-100 dark:text-zinc-900"
              : "hover:bg-zinc-100 dark:hover:bg-zinc-800 text-zinc-700 dark:text-zinc-300",
          ].join(" ")}
        >
          {label}
        </button>
      ))}
    </div>
  );
}
