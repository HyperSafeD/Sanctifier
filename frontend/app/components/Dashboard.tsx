"use client";

import React from "react";
import type { Finding, AnalysisReport, Severity } from "../types";
import type { DashboardTab } from "../providers/DashboardProvider";
import { SeverityFilter } from "./SeverityFilter";
import { FindingsList } from "./FindingsList";
import { SanctityScore } from "./SanctityScore";
import { ErrorBoundary } from "./ErrorBoundary";
import { ComparisonView } from "./ComparisonView";
import { TrendPanel } from "./TrendPanel";
import type { ScanRecord } from "../lib/scan-history";

// ── Types ─────────────────────────────────────────────────────────────────────

export interface DashboardProps {
  /** Transformed list of findings for the current contract. */
  findings: Finding[];
  /** Currently active tab (findings | callgraph | diff). */
  activeTab: DashboardTab;
  /** Callback when the user clicks a tab. */
  onTabChange: (tab: DashboardTab) => void;
  /** Selected severity filter. */
  severityFilter: Severity | "all";
  /** Callback when the severity filter changes. */
  onSeverityFilterChange: (filter: Severity | "all") => void;
  /** Validated finding-code query (e.g. "S001"), or empty string. */
  codeFilter: string;
  /** Validation error for the code filter input, or null when valid. */
  codeFilterError: string | null;
  /** Current value of the code filter text input (may be partially typed). */
  codeFilterInput: string;
  /** Callback when the code-filter input changes. */
  onCodeFilterChange: (value: string) => void;
  /** Baseline report for the diff tab, or null when not loaded. */
  baselineReport: AnalysisReport | null;
  /** Current report for the diff tab, or null when no report is loaded. */
  currentReport: AnalysisReport | null;
  /** Human-readable name for the current contract. */
  currentContractName: string;
  /** Scan-history records powering the trend chart. */
  trendRecords: ScanRecord[];
  /** Callback to clear the scan history. */
  onClearHistory: () => void;
  /** Called with a source getter for a given finding. */
  getSource?: (finding: Finding) => string | undefined;
  /** Called to get the file name for a given finding. */
  getFileName?: (finding: Finding) => string | undefined;
  /** Raw text for the baseline JSON textarea (diff tab). */
  baselineJsonInput: string;
  /** Callback when the baseline textarea changes. */
  onBaselineJsonInputChange: (value: string) => void;
  /** Slot for the call-graph panel — provided by the page (dynamic import). */
  callGraphPanel?: React.ReactNode;
}

// ── Component ─────────────────────────────────────────────────────────────────

/**
 * `Dashboard` is the pure presentational layer for the analysis dashboard.
 *
 * It owns no state of its own — all values are passed as props so the
 * component is fully testable without mounting context providers or a router.
 * The parent page (`app/dashboard/page.tsx`) wires it up to
 * `DashboardProvider` and `WorkspaceProvider`.
 */
export function Dashboard({
  findings,
  activeTab,
  onTabChange,
  severityFilter,
  onSeverityFilterChange,
  codeFilter,
  codeFilterError,
  codeFilterInput,
  onCodeFilterChange,
  baselineReport,
  currentReport,
  currentContractName,
  trendRecords,
  onClearHistory,
  getSource,
  getFileName,
  baselineJsonInput,
  onBaselineJsonInputChange,
  callGraphPanel,
}: DashboardProps) {
  const tabClass = (tab: DashboardTab) =>
    `px-4 py-2 text-sm font-medium border-b-2 transition-colors focus:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-zinc-400 ${
      activeTab === tab
        ? "border-zinc-900 dark:border-zinc-100 text-zinc-900 dark:text-zinc-100"
        : "border-transparent text-zinc-500 hover:text-zinc-700 dark:hover:text-zinc-300"
    }`;

  return (
    <div className="flex-1 space-y-8">
      {/* Summary row */}
      <section className="grid grid-cols-1 md:grid-cols-2 gap-6">
        <ErrorBoundary>
          <SanctityScore findings={findings} />
        </ErrorBoundary>
        <ErrorBoundary>
          <TrendPanel records={trendRecords} onClear={onClearHistory} />
        </ErrorBoundary>
      </section>

      {/* Tab bar */}
      <div
        className="flex gap-2 border-b border-zinc-200 dark:border-zinc-700"
        role="tablist"
        aria-label="Analysis view tabs"
      >
        <button
          onClick={() => onTabChange("findings")}
          role="tab"
          aria-selected={activeTab === "findings"}
          aria-controls="dashboard-findings-panel"
          id="dashboard-findings-tab"
          className={tabClass("findings")}
        >
          Findings
        </button>
        <button
          onClick={() => onTabChange("callgraph")}
          role="tab"
          aria-selected={activeTab === "callgraph"}
          aria-controls="dashboard-callgraph-panel"
          id="dashboard-callgraph-tab"
          className={tabClass("callgraph")}
        >
          Call Graph
        </button>
        <button
          onClick={() => onTabChange("diff")}
          role="tab"
          aria-selected={activeTab === "diff"}
          aria-controls="dashboard-diff-panel"
          id="dashboard-diff-tab"
          className={tabClass("diff")}
        >
          Diff
        </button>
      </div>

      {/* Findings tab */}
      {activeTab === "findings" && (
        <>
          <section>
            <h2 className="text-lg font-semibold mb-4">Filter Findings</h2>
            <div className="space-y-4">
              <SeverityFilter
                selected={severityFilter}
                onChange={onSeverityFilterChange}
              />
              <div className="max-w-xs">
                <label
                  htmlFor="dashboard-code-filter"
                  className="mb-1 block text-sm font-medium"
                >
                  Search by finding code
                </label>
                <input
                  id="dashboard-code-filter"
                  type="text"
                  value={codeFilterInput}
                  onChange={(e) => onCodeFilterChange(e.target.value)}
                  placeholder="S001"
                  inputMode="text"
                  autoCapitalize="characters"
                  autoComplete="off"
                  spellCheck={false}
                  aria-invalid={Boolean(codeFilterError)}
                  aria-describedby="dashboard-code-filter-help"
                  className="w-full rounded-lg border border-zinc-300 bg-white px-3 py-2 font-mono text-sm outline-none transition focus-visible:ring-2 focus-visible:ring-zinc-400 dark:border-zinc-600 dark:bg-zinc-950"
                />
                <p
                  id="dashboard-code-filter-help"
                  className={`mt-1 text-xs ${
                    codeFilterError
                      ? "text-red-600 dark:text-red-400"
                      : "text-zinc-500 dark:text-zinc-400"
                  }`}
                >
                  {codeFilterError ?? "Use exact finding codes like S001, S012, or S020."}
                </p>
              </div>
            </div>
          </section>

          <section
            id="dashboard-findings-panel"
            role="tabpanel"
            aria-labelledby="dashboard-findings-tab"
          >
            <h2 className="text-lg font-semibold mb-4">Findings</h2>
            <ErrorBoundary>
              <FindingsList
                findings={findings}
                severityFilter={severityFilter}
                codeFilter={codeFilterError ? "" : codeFilter}
                getSource={getSource}
                getFileName={getFileName}
              />
            </ErrorBoundary>
          </section>
        </>
      )}

      {/* Call graph tab */}
      {activeTab === "callgraph" && (
        <section
          id="dashboard-callgraph-panel"
          role="tabpanel"
          aria-labelledby="dashboard-callgraph-tab"
        >
          <ErrorBoundary>{callGraphPanel}</ErrorBoundary>
        </section>
      )}

      {/* Diff tab */}
      {activeTab === "diff" && (
        <section
          id="dashboard-diff-panel"
          role="tabpanel"
          aria-labelledby="dashboard-diff-tab"
        >
          <div className="space-y-6">
            <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
              <div>
                <label
                  htmlFor="dashboard-baseline-json-input"
                  className="mb-1 block text-sm font-medium"
                >
                  Baseline Report (JSON)
                </label>
                <textarea
                  id="dashboard-baseline-json-input"
                  value={baselineJsonInput}
                  onChange={(e) => onBaselineJsonInputChange(e.target.value)}
                  placeholder="Paste baseline JSON report here..."
                  rows={4}
                  className="w-full rounded-lg border border-zinc-300 bg-white px-3 py-2 font-mono text-xs outline-none transition focus-visible:ring-2 focus-visible:ring-zinc-400 dark:border-zinc-600 dark:bg-zinc-950"
                />
              </div>
              <div>
                <label className="mb-1 block text-sm font-medium">Current Report</label>
                <div className="w-full rounded-lg border border-zinc-300 dark:border-zinc-600 bg-white dark:bg-zinc-950 px-3 py-2 font-mono text-xs min-h-[5rem] text-zinc-500 dark:text-zinc-400">
                  {currentReport
                    ? "Current report loaded from the active contract."
                    : "No current report loaded. Upload a contract first."}
                </div>
              </div>
            </div>
            <ErrorBoundary>
              <ComparisonView
                baselineReport={baselineReport}
                currentReport={currentReport}
                baselineName="Baseline"
                currentName={currentContractName}
              />
            </ErrorBoundary>
          </div>
        </section>
      )}

      {/* Empty-state messages */}
      {findings.length === 0 && activeTab === "findings" && (
        <p className="text-center text-zinc-500 dark:text-zinc-400 py-8">
          No findings to display. Load a report to view results.
        </p>
      )}
    </div>
  );
}
