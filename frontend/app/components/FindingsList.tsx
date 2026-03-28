"use client";

import { useState, useMemo, useEffect } from "react";
import type { Finding, Severity } from "../types";
import { CodeSnippet } from "./CodeSnippet";

interface FindingsListProps {
  findings: Finding[];
  severityFilter: Severity | "all";
}

const PAGE_SIZE = 20;

const severityColors: Record<Severity, string> = {
  critical: "bg-red-500/10 border-red-500/50 text-red-700 dark:text-red-400 theme-high-contrast:bg-black theme-high-contrast:border-white theme-high-contrast:text-white",
  high: "bg-orange-500/10 border-orange-500/50 text-orange-700 dark:text-orange-400 theme-high-contrast:bg-black theme-high-contrast:border-white theme-high-contrast:text-white",
  medium: "bg-amber-500/10 border-amber-500/50 text-amber-700 dark:text-amber-400 theme-high-contrast:bg-black theme-high-contrast:border-white theme-high-contrast:text-white",
  low: "bg-zinc-500/10 border-zinc-500/50 text-zinc-700 dark:text-zinc-400 theme-high-contrast:bg-black theme-high-contrast:border-white theme-high-contrast:text-yellow-300",
};

const severityLabels: Record<Severity, string> = {
  critical: "Critical severity",
  high: "High severity",
  medium: "Medium severity",
  low: "Low severity",
};

export function FindingsList({ findings, severityFilter }: FindingsListProps) {
  const [visibleCount, setVisibleCount] = useState(PAGE_SIZE);

  const filtered = useMemo(() => {
    const results =
      severityFilter === "all"
        ? findings
        : findings.filter((f) => f.severity === severityFilter);
    return results;
  }, [findings, severityFilter]);

  // Reset visible count when filter changes
  useEffect(() => {
    setVisibleCount(PAGE_SIZE);
  }, [severityFilter, findings]);

  const visibleFindings = filtered.slice(0, visibleCount);
  const hasMore = filtered.length > visibleCount;

  return (
    <div className="space-y-4">
      {filtered.length === 0 ? (
        <p className="text-zinc-500 dark:text-zinc-400 theme-high-contrast:text-white py-8 text-center">
          No findings match the selected filter.
        </p>
      ) : (
        <>
          {visibleFindings.map((f) => (
            <div
              key={f.id}
              className={`rounded-lg border p-4 ${severityColors[f.severity]}`}
            >
              <div className="flex items-start justify-between gap-4">
                <div className="min-w-0 flex-1">
                  <span className="text-xs font-semibold uppercase tracking-wide opacity-80">
                    {f.category}
                  </span>
                  <h3 className="mt-1 font-medium">{f.title}</h3>
                  <p className="mt-1 text-sm opacity-90">{f.location}</p>
                  {f.suggestion && (
                    <p className="mt-2 text-sm italic">💡 {f.suggestion}</p>
                  )}
                </div>
                <div className="shrink-0 flex items-center gap-2">
                  <span
                    className={`rounded px-2 py-1 text-xs font-medium border ${severityColors[f.severity]}`}
                    aria-label={severityLabels[f.severity]}
                  >
                    {f.severity}
                  </span>
                  <span className="font-mono text-xs rounded border border-zinc-300/70 dark:border-zinc-600 px-2 py-1 text-zinc-700 dark:text-zinc-300 theme-high-contrast:border-white theme-high-contrast:text-white">
                    {f.code}
                  </span>
                </div>
              </div>
              {f.snippet && (
                <div className="mt-3">
                  <CodeSnippet code={f.snippet} highlightLine={f.line} />
                </div>
              )}
            </div>
          ))}
          {hasMore && (
            <div className="flex justify-center pt-4">
              <button
                onClick={() => setVisibleCount((prev) => prev + PAGE_SIZE)}
                className="rounded-lg border border-zinc-300 dark:border-zinc-600 px-4 py-2 text-sm font-medium hover:bg-zinc-100 dark:hover:bg-zinc-800 transition-colors"
              >
                Show More ({filtered.length - visibleCount} remaining)
              </button>
            </div>
          )}
        </>
      )}
    </div>
  );
}
