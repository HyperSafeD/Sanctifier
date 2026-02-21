"use client";

import { useMemo } from "react";
import type { Finding, Severity } from "../types";

interface SummaryChartProps {
  findings: Finding[];
}

export function SummaryChart({ findings }: SummaryChartProps) {
  const counts = useMemo(() => {
    const s: Record<Severity, number> = {
      critical: 0,
      high: 0,
      medium: 0,
      low: 0,
    };
    findings.forEach((f) => {
      s[f.severity]++;
    });
    return s;
  }, [findings]);

  const total = findings.length;
  const max = Math.max(...Object.values(counts), 1);

  const bars: { label: Severity; count: number; color: string }[] = [
    { label: "critical", count: counts.critical, color: "bg-red-500" },
    { label: "high", count: counts.high, color: "bg-orange-500" },
    { label: "medium", count: counts.medium, color: "bg-amber-500" },
    { label: "low", count: counts.low, color: "bg-zinc-500" },
  ];

  return (
    <div className="rounded-lg border border-zinc-200 dark:border-zinc-700 bg-white dark:bg-zinc-900 p-4">
      <h3 className="text-sm font-semibold text-zinc-700 dark:text-zinc-300 mb-4">
        Findings by Severity
      </h3>
      <div className="space-y-3">
        {bars.map(({ label, count, color }) => (
          <div key={label} className="flex items-center gap-3">
            <span className="w-20 text-xs font-medium capitalize">{label}</span>
            <div className="flex-1 h-6 bg-zinc-100 dark:bg-zinc-800 rounded overflow-hidden">
              <div
                className={`h-full ${color} transition-all`}
                style={{ width: `${(count / max) * 100}%` }}
              />
            </div>
            <span className="w-8 text-right text-sm">{count}</span>
          </div>
        ))}
      </div>
      <p className="mt-3 text-xs text-zinc-500 dark:text-zinc-400">
        Total: {total} findings
      </p>
    </div>
  );
}
