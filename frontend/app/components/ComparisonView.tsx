"use client";

import { useMemo, useState, useCallback } from "react";
import type { AnalysisReport, ReportDiff, DiffStatus } from "../types";
import { computeReportDiff, exportDiffAsJson } from "../lib/diff";

interface ComparisonViewProps {
  baselineReport: AnalysisReport | null;
  currentReport: AnalysisReport | null;
  baselineName?: string;
  currentName?: string;
}

const diffStatusColors: Record<DiffStatus, { bg: string; border: string; text: string; label: string }> = {
  added: {
    bg: "bg-red-50 dark:bg-red-950/30",
    border: "border-red-400 dark:border-red-600",
    text: "text-red-700 dark:text-red-400",
    label: "Regressed",
  },
  removed: {
    bg: "bg-green-50 dark:bg-green-950/30",
    border: "border-green-400 dark:border-green-600",
    text: "text-green-700 dark:text-green-400",
    label: "Fixed",
  },
  unchanged: {
    bg: "bg-zinc-50 dark:bg-zinc-900/50",
    border: "border-zinc-300 dark:border-zinc-600",
    text: "text-zinc-600 dark:text-zinc-400",
    label: "Unchanged",
  },
  severity_changed: {
    bg: "bg-amber-50 dark:bg-amber-950/30",
    border: "border-amber-400 dark:border-amber-600",
    text: "text-amber-700 dark:text-amber-400",
    label: "Severity Changed",
  },
};

const severityBadgeColors: Record<string, string> = {
  critical: "bg-red-500 text-white",
  high: "bg-orange-500 text-white",
  medium: "bg-amber-500 text-white",
  low: "bg-zinc-400 text-white",
};

export function ComparisonView({
  baselineReport,
  currentReport,
  baselineName = "Baseline",
  currentName = "Current",
}: ComparisonViewProps) {
  const [sortBy, setSortBy] = useState<"status" | "severity" | "code">("status");
  const [statusFilter, setStatusFilter] = useState<DiffStatus | "all">("all");

  const diff: ReportDiff | null = useMemo(() => {
    return computeReportDiff(baselineReport, currentReport, baselineName, currentName);
  }, [baselineReport, currentReport, baselineName, currentName]);

  const triggerExport = useCallback(() => {
    if (!diff) return;
    const merged = exportDiffAsJson(diff);
    const blob = new Blob([JSON.stringify(merged, null, 2)], { type: "application/json" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = `sanctifier-diff-${baselineName}-vs-${currentName}.json`;
    a.click();
    URL.revokeObjectURL(url);
  }, [diff, baselineName, currentName]);

  const sorted = useMemo(() => {
    if (!diff) return [];
    let items = [...diff.diffFindings];
    if (statusFilter !== "all") {
      items = items.filter((d) => d.status === statusFilter);
    }
    const severityOrder: Record<string, number> = { critical: 0, high: 1, medium: 2, low: 3 };
    switch (sortBy) {
      case "status": {
        const statusOrder: Record<string, number> = { added: 0, severity_changed: 1, removed: 2, unchanged: 3 };
        items.sort((a, b) => (statusOrder[a.status] ?? 99) - (statusOrder[b.status] ?? 99));
        break;
      }
      case "severity":
        items.sort((a, b) => (severityOrder[a.finding.severity] ?? 99) - (severityOrder[b.finding.severity] ?? 99));
        break;
      case "code":
        items.sort((a, b) => a.finding.code.localeCompare(b.finding.code));
        break;
    }
    return items;
  }, [diff, sortBy, statusFilter]);

  if (!diff) {
    return (
      <div className="rounded-lg border border-zinc-200 dark:border-zinc-700 bg-white dark:bg-zinc-900 p-8 text-center text-zinc-500">
        Load two reports to compare them.
      </div>
    );
  }

  const hasChanges = diff.addedCount > 0 || diff.removedCount > 0 || diff.severityChangedCount > 0;

  return (
    <div className="space-y-6">
      {/* Summary cards */}
      <div className="grid grid-cols-2 sm:grid-cols-4 gap-4">
        <div className="rounded-lg border border-zinc-200 dark:border-zinc-700 bg-white dark:bg-zinc-900 p-4">
          <p className="text-xs font-medium uppercase tracking-wide text-zinc-500">Baseline</p>
          <p className="mt-1 text-2xl font-bold">{diff.baselineFindings.length}</p>
          <p className="text-xs text-zinc-400">{diff.baselineName}</p>
        </div>
        <div className="rounded-lg border border-zinc-200 dark:border-zinc-700 bg-white dark:bg-zinc-900 p-4">
          <p className="text-xs font-medium uppercase tracking-wide text-zinc-500">Current</p>
          <p className="mt-1 text-2xl font-bold">{diff.currentFindings.length}</p>
          <p className="text-xs text-zinc-400">{diff.currentName}</p>
        </div>
        <div className="rounded-lg border border-red-200 dark:border-red-800 bg-red-50 dark:bg-red-950/30 p-4">
          <p className="text-xs font-medium uppercase tracking-wide text-red-600 dark:text-red-400">Regressions</p>
          <p className="mt-1 text-2xl font-bold text-red-700 dark:text-red-400">{diff.addedCount}</p>
          <p className="text-xs text-red-500">+ {diff.severityChangedCount} severity changes</p>
        </div>
        <div className="rounded-lg border border-green-200 dark:border-green-800 bg-green-50 dark:bg-green-950/30 p-4">
          <p className="text-xs font-medium uppercase tracking-wide text-green-600 dark:text-green-400">Fixed</p>
          <p className="mt-1 text-2xl font-bold text-green-700 dark:text-green-400">{diff.removedCount}</p>
          <p className="text-xs text-green-500">{diff.unchangedCount} unchanged</p>
        </div>
      </div>

      {/* Controls */}
      <div className="flex flex-wrap items-center gap-4">
        <div className="flex items-center gap-2">
          <label htmlFor="diff-status-filter" className="text-sm font-medium text-zinc-600 dark:text-zinc-400">
            Status
          </label>
          <select
            id="diff-status-filter"
            value={statusFilter}
            onChange={(e) => setStatusFilter(e.target.value as DiffStatus | "all")}
            className="rounded-lg border border-zinc-300 dark:border-zinc-600 bg-white dark:bg-zinc-900 px-3 py-1.5 text-sm"
          >
            <option value="all">All</option>
            <option value="added">Regressions</option>
            <option value="removed">Fixed</option>
            <option value="severity_changed">Severity Changed</option>
            <option value="unchanged">Unchanged</option>
          </select>
        </div>
        <div className="flex items-center gap-2">
          <label htmlFor="diff-sort" className="text-sm font-medium text-zinc-600 dark:text-zinc-400">
            Sort
          </label>
          <select
            id="diff-sort"
            value={sortBy}
            onChange={(e) => setSortBy(e.target.value as "status" | "severity" | "code")}
            className="rounded-lg border border-zinc-300 dark:border-zinc-600 bg-white dark:bg-zinc-900 px-3 py-1.5 text-sm"
          >
            <option value="status">By status</option>
            <option value="severity">By severity</option>
            <option value="code">By code</option>
          </select>
        </div>
        <button
          onClick={triggerExport}
          className="ml-auto rounded-lg border border-zinc-300 dark:border-zinc-600 bg-white dark:bg-zinc-900 px-4 py-1.5 text-sm font-medium hover:bg-zinc-100 dark:hover:bg-zinc-800 transition-colors"
        >
          Export Diff as JSON
        </button>
      </div>

      {/* Findings list */}
      {sorted.length === 0 && (
        <p className="text-center text-zinc-500 py-8">
          {hasChanges
            ? "No findings match the selected filter."
            : "No differences between the two reports — they are identical."}
        </p>
      )}

      <div className="space-y-3">
        {sorted.map((d, idx) => {
          const colors = diffStatusColors[d.status];
          const sevColor = severityBadgeColors[d.finding.severity] ?? "bg-zinc-400 text-white";
          return (
            <div
              key={`${d.finding.id}-${idx}`}
              className={`rounded-lg border-2 p-4 ${colors.bg} ${colors.border}`}
            >
              <div className="flex items-start justify-between gap-4">
                <div className="min-w-0 flex-1">
                  <div className="flex items-center gap-2 mb-1">
                    <span className={`text-[10px] font-bold uppercase tracking-wider px-1.5 py-0.5 rounded ${colors.text} border ${colors.border}`}>
                      {colors.label}
                    </span>
                    <span className="text-xs font-semibold uppercase tracking-wide opacity-70">
                      {d.finding.category}
                    </span>
                  </div>
                  <h3 className="font-medium">{d.finding.title}</h3>
                  <p className="mt-0.5 text-sm opacity-80">{d.finding.location}</p>
                  {d.finding.suggestion && (
                    <p className="mt-1 text-sm italic opacity-70">💡 {d.finding.suggestion}</p>
                  )}
                  {d.status === "severity_changed" && d.previousSeverity && (
                    <p className="mt-1 text-xs text-amber-600 dark:text-amber-400">
                      Severity changed from {d.previousSeverity} to {d.finding.severity}
                    </p>
                  )}
                </div>
                <div className="shrink-0 flex items-center gap-2">
                  <span className={`rounded px-2 py-1 text-xs font-medium ${sevColor}`}>
                    {d.finding.severity}
                  </span>
                  <span className="font-mono text-xs rounded border border-zinc-300/70 dark:border-zinc-600 px-2 py-1 text-zinc-700 dark:text-zinc-300">
                    {d.finding.code}
                  </span>
                </div>
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}
