"use client";

import { useMemo } from "react";
import Link from "next/link";
import { Download, FileJson, FileText } from "lucide-react";
import { Breadcrumbs } from "../components/Breadcrumbs";
import { useWorkspace } from "../providers/WorkspaceProvider";
import { useToast } from "../providers/ToastProvider";
import { buildAuditReport, SEVERITIES, type SeverityCounts } from "../lib/audit-report";
import { exportToPdf } from "../lib/export-pdf";
import { findingsToSarif } from "../lib/sarif";
import type { Severity } from "../types";

const SEVERITY_BAR: Record<Severity, string> = {
  critical: "bg-red-600",
  high: "bg-orange-500",
  medium: "bg-amber-400",
  low: "bg-emerald-500",
};

const SEVERITY_TEXT: Record<Severity, string> = {
  critical: "text-red-700 dark:text-red-400",
  high: "text-orange-700 dark:text-orange-400",
  medium: "text-amber-700 dark:text-amber-400",
  low: "text-emerald-700 dark:text-emerald-400",
};

function SeverityBar({ counts, total }: { counts: SeverityCounts; total: number }) {
  if (total === 0) {
    return <p className="text-xs text-zinc-500">No findings</p>;
  }
  const summary = SEVERITIES.map((s) => `${counts[s]} ${s}`).join(", ");
  return (
    <div role="img" aria-label={`Severity distribution: ${summary}`} className="flex h-2.5 w-full overflow-hidden rounded-full bg-zinc-100 dark:bg-zinc-800">
      {SEVERITIES.filter((s) => counts[s] > 0).map((s) => (
        <div key={s} className={SEVERITY_BAR[s]} style={{ width: `${(counts[s] / total) * 100}%` }} />
      ))}
    </div>
  );
}

function downloadSarif(sarif: unknown) {
  const blob = new Blob([JSON.stringify(sarif, null, 2)], { type: "application/sarif+json" });
  const url = URL.createObjectURL(blob);
  const link = document.createElement("a");
  link.href = url;
  link.download = "sanctifier-audit.sarif";
  document.body.appendChild(link);
  link.click();
  link.remove();
  URL.revokeObjectURL(url);
}

export default function AuditPage() {
  const { workspace } = useWorkspace();
  const toast = useToast();
  const report = useMemo(() => (workspace ? buildAuditReport(workspace) : null), [workspace]);

  const handlePdf = async () => {
    if (!report) return;
    try {
      await exportToPdf(report.findings, "Sanctifier Audit Report");
    } catch {
      toast.error("PDF export failed. Please try again.");
    }
  };

  const handleSarif = () => {
    if (!report) return;
    downloadSarif(findingsToSarif(report.findings));
  };

  return (
    <>
      <Breadcrumbs />
      <main className="max-w-6xl mx-auto px-4 sm:px-6 lg:px-8 py-8 space-y-8">
        <header className="flex flex-col gap-4 sm:flex-row sm:items-end sm:justify-between">
          <div>
            <h1 className="text-2xl font-bold text-zinc-900 dark:text-zinc-50">Audit Report</h1>
            <p className="mt-1 text-sm text-zinc-600 dark:text-zinc-400">
              Workspace-wide summary of the analysis results currently loaded in the dashboard.
            </p>
          </div>
          <div className="flex gap-2">
            <button
              type="button"
              onClick={handlePdf}
              disabled={!report || report.findings.length === 0}
              className="inline-flex items-center gap-2 rounded-lg bg-zinc-900 px-4 py-2 text-sm font-medium text-white hover:bg-zinc-700 disabled:cursor-not-allowed disabled:opacity-50 dark:bg-zinc-100 dark:text-zinc-900 dark:hover:bg-zinc-300"
            >
              <FileText className="h-4 w-4" aria-hidden="true" />
              Export PDF
            </button>
            <button
              type="button"
              onClick={handleSarif}
              disabled={!report || report.findings.length === 0}
              className="inline-flex items-center gap-2 rounded-lg border border-zinc-300 px-4 py-2 text-sm font-medium text-zinc-700 hover:bg-zinc-50 disabled:cursor-not-allowed disabled:opacity-50 dark:border-zinc-700 dark:text-zinc-300 dark:hover:bg-zinc-800"
            >
              <FileJson className="h-4 w-4" aria-hidden="true" />
              Export SARIF
            </button>
          </div>
        </header>

        {!report ? (
          <div className="rounded-xl border border-dashed border-zinc-300 dark:border-zinc-700 p-10 text-center">
            <Download className="mx-auto h-8 w-8 text-zinc-400" aria-hidden="true" />
            <p className="mt-3 font-medium text-zinc-900 dark:text-zinc-100">No workspace loaded</p>
            <p className="mt-1 text-sm text-zinc-500">
              Load a workspace report in the{" "}
              <Link href="/dashboard" className="underline">
                dashboard
              </Link>{" "}
              or scan a contract to generate an audit report.
            </p>
          </div>
        ) : (
          <>
            {/* Executive summary */}
            <section aria-labelledby="summary-heading" className="rounded-xl border border-zinc-200 dark:border-zinc-800 bg-white dark:bg-zinc-900 p-6">
              <h2 id="summary-heading" className="text-lg font-semibold text-zinc-900 dark:text-zinc-50">
                Executive summary
              </h2>
              <div className="mt-4 grid grid-cols-2 gap-4 md:grid-cols-4">
                <div>
                  <p className="text-xs uppercase tracking-wide text-zinc-500">Overall risk</p>
                  <p className="text-2xl font-bold text-zinc-900 dark:text-zinc-50">{report.riskLabel}</p>
                  <p className="text-xs text-zinc-500">Sanctity score {report.score}/100</p>
                </div>
                <div>
                  <p className="text-xs uppercase tracking-wide text-zinc-500">Contracts</p>
                  <p className="text-2xl font-bold text-zinc-900 dark:text-zinc-50">{report.contracts.length}</p>
                </div>
                <div>
                  <p className="text-xs uppercase tracking-wide text-zinc-500">Total findings</p>
                  <p className="text-2xl font-bold text-zinc-900 dark:text-zinc-50">{report.totals.all}</p>
                </div>
                <div>
                  <p className="text-xs uppercase tracking-wide text-zinc-500">Critical / High</p>
                  <p className="text-2xl font-bold text-zinc-900 dark:text-zinc-50">
                    {report.totals.critical} / {report.totals.high}
                  </p>
                </div>
              </div>
              <div className="mt-5">
                <SeverityBar counts={report.totals} total={report.totals.all} />
                <ul className="mt-2 flex flex-wrap gap-x-4 gap-y-1 text-xs">
                  {SEVERITIES.map((s) => (
                    <li key={s} className={`capitalize ${SEVERITY_TEXT[s]}`}>
                      {s}: {report.totals[s]}
                    </li>
                  ))}
                </ul>
              </div>
            </section>

            {/* Per-contract breakdown */}
            <section aria-labelledby="contracts-heading">
              <h2 id="contracts-heading" className="text-lg font-semibold text-zinc-900 dark:text-zinc-50">
                Per-contract breakdown
              </h2>
              <ul className="mt-3 grid grid-cols-1 gap-4 md:grid-cols-2 lg:grid-cols-3">
                {report.contracts.map((contract) => (
                  <li
                    key={contract.name}
                    className="rounded-xl border border-zinc-200 dark:border-zinc-800 bg-white dark:bg-zinc-900 p-4"
                  >
                    <div className="flex items-center justify-between gap-2">
                      <h3 className="truncate font-medium text-zinc-900 dark:text-zinc-50">{contract.name}</h3>
                      <span className="text-xs text-zinc-500">Score {contract.score}</span>
                    </div>
                    <p className="mt-1 text-sm text-zinc-600 dark:text-zinc-400">
                      {contract.total} {contract.total === 1 ? "finding" : "findings"}
                    </p>
                    <div className="mt-3">
                      <SeverityBar counts={contract.counts} total={contract.total} />
                    </div>
                  </li>
                ))}
              </ul>
            </section>

            {/* Top critical findings */}
            <section aria-labelledby="top-heading" className="rounded-xl border border-zinc-200 dark:border-zinc-800 bg-white dark:bg-zinc-900 p-6">
              <h2 id="top-heading" className="text-lg font-semibold text-zinc-900 dark:text-zinc-50">
                Top 10 critical findings
              </h2>
              {report.topFindings.length === 0 ? (
                <p className="mt-3 text-sm text-zinc-500">No findings to report.</p>
              ) : (
                <ol className="mt-3 divide-y divide-zinc-100 dark:divide-zinc-800">
                  {report.topFindings.map((finding) => (
                    <li key={finding.id} className="flex items-start justify-between gap-4 py-2.5 text-sm">
                      <div className="min-w-0">
                        <p className="truncate font-medium text-zinc-900 dark:text-zinc-100">
                          {finding.code} · {finding.title}
                        </p>
                        <p className="truncate font-mono text-xs text-zinc-500">{finding.location}</p>
                      </div>
                      <span className={`shrink-0 text-xs font-semibold capitalize ${SEVERITY_TEXT[finding.severity]}`}>
                        {finding.severity}
                      </span>
                    </li>
                  ))}
                </ol>
              )}
            </section>

            {/* Remediation timeline */}
            <section aria-labelledby="timeline-heading" className="rounded-xl border border-zinc-200 dark:border-zinc-800 bg-white dark:bg-zinc-900 p-6">
              <h2 id="timeline-heading" className="text-lg font-semibold text-zinc-900 dark:text-zinc-50">
                Remediation timeline
              </h2>
              {report.timeline.length === 0 ? (
                <p className="mt-3 text-sm text-zinc-500">Nothing to remediate.</p>
              ) : (
                <ol className="mt-3 space-y-3">
                  {report.timeline.map((step) => (
                    <li key={step.severity} className="flex items-center gap-3 text-sm">
                      <span className={`h-2.5 w-2.5 shrink-0 rounded-full ${SEVERITY_BAR[step.severity]}`} aria-hidden="true" />
                      <span className="text-zinc-900 dark:text-zinc-100">
                        <span className="font-medium capitalize">{step.severity}</span> ({step.count}) —{" "}
                        {step.window}
                      </span>
                    </li>
                  ))}
                </ol>
              )}
            </section>
          </>
        )}
      </main>
    </>
  );
}
