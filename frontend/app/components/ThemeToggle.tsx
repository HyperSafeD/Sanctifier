"use client";

import { useEffect, useState } from "react";

type Theme = "light" | "dark" | "high-contrast";

export function ThemeToggle() {
  const [theme, setTheme] = useState<Theme>("light");

  useEffect(() => {
    // Check initial state
    const isDark = document.documentElement.classList.contains("dark");
    const isHC = document.documentElement.classList.contains("theme-high-contrast");
    
    if (isHC) {
      setTheme("high-contrast");
    } else if (isDark || window.matchMedia("(prefers-color-scheme: dark)").matches) {
      setTheme("dark");
      document.documentElement.classList.add("dark");
    }
  }, []);

  const cycleTheme = () => {
    const root = document.documentElement;
    
    if (theme === "light") {
      root.classList.add("dark");
      setTheme("dark");
    } else if (theme === "dark") {
      root.classList.remove("dark");
      root.classList.add("theme-high-contrast");
      setTheme("high-contrast");
    } else {
      root.classList.remove("theme-high-contrast");
      setTheme("light");
    }
  };

  const getLabel = () => {
    if (theme === "light") return "☀️ Light";
    if (theme === "dark") return "🌙 Dark";
    return "⬛ High Contrast";
  };

  return (
    <button
      onClick={cycleTheme}
      className="rounded-lg border border-zinc-300 dark:border-zinc-600 px-3 py-2 text-sm hover:bg-zinc-100 dark:hover:bg-zinc-800 theme-high-contrast:border-white theme-high-contrast:hover:bg-zinc-900"
      aria-label="Toggle theme"
    >
      {getLabel()}
    </button>
  );
}
