"use client";

import { useState, useCallback } from "react";
import type { AnalysisReport, Finding, Severity } from "../types";
import { transformReport } from "../lib/transform";
import { exportToPdf } from "../lib/export-pdf";
import { SeverityFilter } from "../components/SeverityFilter";
import { FindingsList } from "../components/FindingsList";
import { SummaryChart } from "../components/SummaryChart";
import { ThemeToggle } from "../components/ThemeToggle";
import Link from "next/link";

const SAMPLE_JSON = `{
  "size_warnings": [],
  "unsafe_patterns": [],
  "auth_gaps": [],
  "panic_issues": [],
  "arithmetic_issues": []
}`;

export default function DashboardPage() {
  const [findings, setFindings] = useState<Finding[]>([]);
  const [severityFilter, setSeverityFilter] = useState<Severity | "all">("all");
  const [error, setError] = useState<string | null>(null);
  const [jsonInput, setJsonInput] = useState("");

  const loadReport = useCallback(() => {
    setError(null);
    try {
      const parsed = JSON.parse(jsonInput || SAMPLE_JSON) as AnalysisReport;
      setFindings(transformReport(parsed));
    } catch (e) {
      setError(e instanceof Error ? e.message : "Invalid JSON");
      setFindings([]);
    }
  }, [jsonInput]);

  const handleFileUpload = useCallback((e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;
    const reader = new FileReader();
    reader.onload = (ev) => {
      const text = ev.target?.result as string;
      setJsonInput(text);
      setError(null);
      try {
        const parsed = JSON.parse(text) as AnalysisReport;
        setFindings(transformReport(parsed));
      } catch (err) {
        setError(err instanceof Error ? err.message : "Invalid JSON");
      }
    };
    reader.readAsText(file);
    e.target.value = "";
  }, []);

  return (
    <div className="min-h-screen bg-zinc-50 dark:bg-zinc-950 text-zinc-900 dark:text-zinc-100">
      <header className="border-b border-zinc-200 dark:border-zinc-800 bg-white dark:bg-zinc-900 px-6 py-4 flex items-center justify-between">
        <div className="flex items-center gap-6">
          <Link href="/" className="font-bold text-lg">
            Sanctifier
          </Link>
          <span className="text-zinc-500 dark:text-zinc-400">Security Dashboard</span>
        </div>
        <ThemeToggle />
      </header>

      <main className="max-w-6xl mx-auto px-6 py-8 space-y-8">
        <section className="rounded-lg border border-zinc-200 dark:border-zinc-800 bg-white dark:bg-zinc-900 p-6">
          <h2 className="text-lg font-semibold mb-4">Load Analysis Report</h2>
          <p className="text-sm text-zinc-600 dark:text-zinc-400 mb-4">
            Paste JSON from <code className="bg-zinc-100 dark:bg-zinc-800 px-1 rounded">sanctifier analyze --format json</code> or upload a file.
          </p>
          <div className="flex flex-wrap gap-4">
            <label className="cursor-pointer rounded-lg border border-zinc-300 dark:border-zinc-600 px-4 py-2 text-sm hover:bg-zinc-100 dark:hover:bg-zinc-800">
              Upload JSON
              <input
                type="file"
                accept=".json"
                className="hidden"
                onChange={handleFileUpload}
              />
            </label>
            <button
              onClick={loadReport}
              className="rounded-lg bg-zinc-900 dark:bg-zinc-100 text-white dark:text-zinc-900 px-4 py-2 text-sm font-medium hover:bg-zinc-800 dark:hover:bg-zinc-200"
            >
              Parse JSON
            </button>
            <button
              onClick={() => {
                exportToPdf(findings);
              }}
              disabled={findings.length === 0}
              className="rounded-lg border border-zinc-300 dark:border-zinc-600 px-4 py-2 text-sm disabled:opacity-50 hover:bg-zinc-100 dark:hover:bg-zinc-800"
            >
              Export PDF
            </button>
          </div>
          {error && (
            <p className="mt-2 text-sm text-red-600 dark:text-red-400">{error}</p>
          )}
          <textarea
            value={jsonInput}
            onChange={(e) => setJsonInput(e.target.value)}
            placeholder={SAMPLE_JSON}
            className="mt-4 w-full h-32 rounded-lg border border-zinc-300 dark:border-zinc-600 bg-white dark:bg-zinc-950 p-3 font-mono text-sm focus:ring-2 focus:ring-zinc-400 dark:focus:ring-zinc-600 outline-none"
          />
        </section>

        {findings.length > 0 && (
          <>
            <section>
              <SummaryChart findings={findings} />
            </section>

            <section>
              <h2 className="text-lg font-semibold mb-4">Filter by Severity</h2>
              <SeverityFilter selected={severityFilter} onChange={setSeverityFilter} />
            </section>

            <section>
              <h2 className="text-lg font-semibold mb-4">Findings</h2>
              <FindingsList findings={findings} severityFilter={severityFilter} />
            </section>
          </>
        )}

        {findings.length === 0 && !error && (
          <p className="text-center text-zinc-500 dark:text-zinc-400 py-12">
            Load a report to view findings.
          </p>
        )}
      </main>
    </div>
  );
}
