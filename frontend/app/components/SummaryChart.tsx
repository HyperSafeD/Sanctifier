"use client";

import { useMemo } from "react";
import type { Finding, Severity } from "../types";
import type { ScanRecord } from "../lib/scan-history";
import { TrendChart } from "./TrendChart";

interface SummaryChartProps {
  findings: Finding[];
  trendRecords?: ScanRecord[];
  onClearHistory?: () => void;
}

const trendColors: Record<string, string> = {
  critical: "#ef4444",
  high: "#f97316",
  medium: "#f59e0b",
  low: "#71717a",
};

export function SummaryChart({ findings, trendRecords = [], onClearHistory }: SummaryChartProps) {
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

  const hasTrend = trendRecords.length > 0;

  return (
    <div className="rounded-lg border border-zinc-200 dark:border-zinc-700 bg-white dark:bg-zinc-900 p-4">
      <h3 className="text-sm font-semibold text-zinc-700 dark:text-zinc-300 mb-4">
        Findings by Severity
      </h3>
      <div className="space-y-3">
        {bars.map(({ label, count, color }) => (
<div key={label} className="flex items-center gap-2 sm:gap-3">
        <span className="w-16 sm:w-20 text-[10px] sm:text-xs font-medium capitalize truncate">{label}</span>
        <div className="flex-1 h-6 bg-zinc-100 dark:bg-zinc-800 rounded overflow-hidden">
          <div
            className={`h-full ${color} transition-all`}
            style={{ width: `${(count / max) * 100}%` }}
            role="progressbar"
            aria-valuenow={count}
            aria-valuemin={0}
            aria-valuemax={max}
            aria-label={`${label} severity: ${count} findings`}
          />
        </div>
        <span className="w-6 sm:w-8 text-right text-xs sm:text-sm">{count}</span>
      </div>
        ))}
      </div>
      <p className="mt-3 text-xs text-zinc-500 dark:text-zinc-400">
        Total: {total} findings
      </p>

      {hasTrend && (
        <div className="mt-6 pt-4 border-t border-zinc-200 dark:border-zinc-700">
          <div className="flex items-center justify-between mb-3">
            <h4 className="text-xs font-semibold text-zinc-600 dark:text-zinc-400 uppercase tracking-wider">
              Severity Trend ({trendRecords.length} scans)
            </h4>
            {onClearHistory && (
              <button
                onClick={onClearHistory}
                className="text-[10px] text-zinc-400 hover:text-red-400 transition-colors"
                aria-label="Clear all scan history"
              >
                Clear history
              </button>
            )}
          </div>
          <div className="grid grid-cols-2 gap-4">
            {(["critical", "high", "medium", "low"] as const).map((sev) => (
              <TrendChart
                key={sev}
                records={trendRecords}
                severity={sev}
                color={trendColors[sev]}
              />
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
