"use client";

import { useMemo } from "react";
import { TrendChart } from "./TrendChart";
import { recentScanRecords, TREND_WINDOW_DAYS } from "../lib/trend";
import type { ScanRecord } from "../lib/scan-history";

const SEVERITIES = [
  { severity: "critical", label: "Critical", color: "#dc2626" },
  { severity: "high", label: "High", color: "#ea580c" },
  { severity: "medium", label: "Medium", color: "#f59e0b" },
  { severity: "low", label: "Low", color: "#22c55e" },
] as const;

/** Findings over the last 30 days, one small chart per severity. */
export function TrendPanel({ records, onClear }: { records: ScanRecord[]; onClear?: () => void }) {
  const recent = useMemo(() => recentScanRecords(records), [records]);

  return (
    <section
      aria-labelledby="trend-heading"
      className="rounded-xl border border-zinc-200 dark:border-zinc-800 bg-white dark:bg-zinc-900 p-6"
    >
      <div className="flex items-center justify-between">
        <h2 id="trend-heading" className="text-lg font-semibold text-zinc-900 dark:text-zinc-50">
          Findings over the last {TREND_WINDOW_DAYS} days
        </h2>
        {onClear && recent.length > 0 && (
          <button
            type="button"
            onClick={onClear}
            className="text-xs text-zinc-500 hover:text-zinc-900 dark:hover:text-zinc-100"
          >
            Clear history
          </button>
        )}
      </div>

      {recent.length < 2 ? (
        <p className="mt-3 text-sm text-zinc-500">
          Run at least two scans to see a trend. {recent.length} scan{recent.length === 1 ? "" : "s"} in the last{" "}
          {TREND_WINDOW_DAYS} days.
        </p>
      ) : (
        <div className="mt-4 grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
          {SEVERITIES.map(({ severity, label, color }) => (
            <div key={severity}>
              <p className="mb-1 text-xs font-medium uppercase tracking-wide text-zinc-500">
                {label}: {recent[recent.length - 1][severity]}
              </p>
              <TrendChart records={recent} severity={severity} color={color} />
            </div>
          ))}
        </div>
      )}
    </section>
  );
}
