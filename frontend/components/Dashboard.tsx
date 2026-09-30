"use client";

import { useState, useCallback, useMemo } from "react";
import type { Severity } from "../app/types";
import { transformReport, normalizeReport } from "../app/lib/transform";
import type { WorkspaceSummary } from "../app/types";
import { SeverityFilter } from "../app/components/SeverityFilter";
import { FindingsList } from "../app/components/FindingsList";
import { SummaryChart } from "../app/components/SummaryChart";
import { SanctityScore } from "../app/components/SanctityScore";

export interface DashboardProps {
  workspaceSummary: WorkspaceSummary | null;
  onSeverityChange?: (severity: Severity | "all") => void;
  className?: string;
}

/**
 * Dashboard component displays security findings and analysis results.
 * Provides severity filtering, summary charts, and detailed findings list.
 *
 * @param workspaceSummary - The workspace analysis report
 * @param onSeverityChange - Optional callback when severity filter changes
 * @param className - Optional CSS class names
 */
export function Dashboard({
  workspaceSummary,
  onSeverityChange,
  className = "",
}: DashboardProps) {
  const [severityFilter, setSeverityFilter] = useState<Severity | "all">("all");

  const handleSeverityChange = useCallback(
    (severity: Severity | "all") => {
      setSeverityFilter(severity);
      onSeverityChange?.(severity);
    },
    [onSeverityChange]
  );

  const findings = useMemo(() => {
    if (!workspaceSummary?.contracts?.[0]?.report) {
      return [];
    }
    const report = normalizeReport(workspaceSummary.contracts[0].report);
    return transformReport(report);
  }, [workspaceSummary]);

  const hasData = workspaceSummary !== null && findings.length > 0;

  return (
    <div
      className={`dashboard-container ${className}`}
      data-testid="dashboard"
      role="region"
      aria-label="Security analysis dashboard"
    >
      {!hasData && (
        <div
          className="empty-state"
          role="status"
          aria-live="polite"
          data-testid="dashboard-empty"
        >
          <p>No analysis data available. Load a report to view findings.</p>
        </div>
      )}

      {hasData && (
        <div className="dashboard-content" data-testid="dashboard-content">
          <section
            className="dashboard-summary"
            aria-labelledby="summary-heading"
          >
            <h2 id="summary-heading" className="sr-only">
              Analysis Summary
            </h2>
            <div className="summary-grid">
              <SanctityScore findings={findings} />
              <SummaryChart findings={findings} />
            </div>
          </section>

          <section
            className="dashboard-filters"
            aria-labelledby="filters-heading"
          >
            <h2 id="filters-heading" className="sr-only">
              Filter Findings
            </h2>
            <SeverityFilter
              selected={severityFilter}
              onChange={handleSeverityChange}
            />
          </section>

          <section
            className="dashboard-findings"
            aria-labelledby="findings-heading"
          >
            <h2 id="findings-heading" className="sr-only">
              Security Findings
            </h2>
            <FindingsList
              findings={findings}
              severityFilter={severityFilter}
              codeFilter=""
            />
          </section>
        </div>
      )}
    </div>
  );
}
