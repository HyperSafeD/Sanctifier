"use client";

import type { Severity } from "../types";

interface SeverityFilterProps {
  selected: Severity | "all";
  onChange: (s: Severity | "all") => void;
}

const labels: Record<Severity | "all", string> = {
  all: "All",
  critical: "Critical",
  high: "High",
  medium: "Medium",
  low: "Low",
};

const colors: Record<Severity, string> = {
  critical: "bg-red-500",
  high: "bg-orange-500",
  medium: "bg-amber-500",
  low: "bg-zinc-500",
};

export function SeverityFilter({ selected, onChange }: SeverityFilterProps) {
  const options: (Severity | "all")[] = ["all", "critical", "high", "medium", "low"];

  return (
    <div className="flex flex-wrap gap-2">
      {options.map((s) => (
        <button
          key={s}
          onClick={() => onChange(s)}
          className={`rounded-lg px-3 py-1.5 text-sm font-medium transition-colors ${
            selected === s
              ? s === "all"
                ? "bg-zinc-800 dark:bg-zinc-700 text-white"
                : `${colors[s as Severity]} text-white`
              : "bg-zinc-200 dark:bg-zinc-800 text-zinc-700 dark:text-zinc-300 hover:bg-zinc-300 dark:hover:bg-zinc-700"
          }`}
        >
          {labels[s]}
        </button>
      ))}
    </div>
  );
}
