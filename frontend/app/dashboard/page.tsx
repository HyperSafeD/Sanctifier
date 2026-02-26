"use client";

import { useState, useCallback, useMemo } from "react";
import type { AnalysisReport, CallGraphNode, CallGraphEdge, Finding, Severity } from "../types";
import { transformReport, extractCallGraph } from "../lib/transform";
import { exportToPdf } from "../lib/export-pdf";
import { SeverityFilter } from "../components/SeverityFilter";
import { FindingsList } from "../components/FindingsList";
import { SummaryChart } from "../components/SummaryChart";
import { SanctityScore } from "../components/SanctityScore";
import { CallGraph } from "../components/CallGraph";
import { ThemeToggle } from "../components/ThemeToggle";
import Link from "next/link";

const SAMPLE_JSON = `{
  "size_warnings": [],
  "unsafe_patterns": [],
  "auth_gaps": [],
  "panic_issues": [],
  "arithmetic_issues": []
}`;

type Tab = "findings" | "callgraph";

export default function DashboardPage() {
  const [findings, setFindings] = useState<Finding[]>([]);
  const [callGraphNodes, setCallGraphNodes] = useState<CallGraphNode[]>([]);
  const [callGraphEdges, setCallGraphEdges] = useState<CallGraphEdge[]>([]);
  const [severityFilter, setSeverityFilter] = useState<Severity | "all">("all");
  const [error, setError] = useState<string | null>(null);
  const [jsonInput, setJsonInput] = useState("");
  const [activeTab, setActiveTab] = useState<Tab>("findings");

  const parseReport = useCallback((text: string) => {
    setError(null);
    try {
      const parsed = JSON.parse(text || SAMPLE_JSON) as AnalysisReport;

      // Handle new CI/CD format with nested "findings" key
      const report = (parsed as Record<string, unknown>).findings
        ? ((parsed as Record<string, unknown>).findings as AnalysisReport)
        : parsed;

      setFindings(transformReport(report));
      const { nodes, edges } = extractCallGraph(report);
      setCallGraphNodes(nodes);
      setCallGraphEdges(edges);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Invalid JSON");
      setFindings([]);
      setCallGraphNodes([]);
      setCallGraphEdges([]);
    }
  }, []);

  const loadReport = useCallback(() => {
    parseReport(jsonInput);
  }, [jsonInput, parseReport]);

  const handleFileUpload = useCallback((e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;
    const reader = new FileReader();
    reader.onload = (ev) => {
      const text = ev.target?.result as string;
      setJsonInput(text);
      parseReport(text);
    };
    reader.readAsText(file);
    e.target.value = "";
  }, [parseReport]);

  const hasData = findings.length > 0;

  return (
    <div className="min-h-screen bg-zinc-50 dark:bg-zinc-950 text-zinc-900 dark:text-zinc-100 theme-high-contrast:bg-black theme-high-contrast:text-white">
      <header className="border-b border-zinc-200 dark:border-zinc-800 theme-high-contrast:border-b-white bg-white dark:bg-zinc-900 theme-high-contrast:bg-black px-6 py-4 flex items-center justify-between">
        <div className="flex items-center gap-6">
          <Link href="/" className="font-bold text-lg theme-high-contrast:text-yellow-300">
            Sanctifier
          </Link>
          <span className="text-zinc-500 dark:text-zinc-400 theme-high-contrast:text-white">Security Dashboard</span>
        </div>
        <div className="flex items-center gap-4">
          <Link
            href="/terminal"
            className="text-sm font-medium text-zinc-600 dark:text-zinc-400 hover:text-zinc-950 dark:hover:text-zinc-50 transition-colors"
          >
            Live Terminal
          </Link>
          <ThemeToggle />
        </div>
      </header>

      <main className="max-w-6xl mx-auto px-6 py-8 space-y-8">
        <section className="rounded-lg border border-zinc-200 dark:border-zinc-800 theme-high-contrast:border-white bg-white dark:bg-zinc-900 theme-high-contrast:bg-black p-6">
          <h2 className="text-lg font-semibold mb-4 theme-high-contrast:text-yellow-300">Load Analysis Report</h2>
          <p className="text-sm text-zinc-600 dark:text-zinc-400 theme-high-contrast:text-white mb-4">
            Paste JSON from <code className="bg-zinc-100 dark:bg-zinc-800 theme-high-contrast:bg-zinc-900 px-1 rounded">sanctifier analyze --format json</code> or upload a file.
          </p>
          <div className="flex flex-wrap gap-4">
            <label className="cursor-pointer rounded-lg border border-zinc-300 dark:border-zinc-600 theme-high-contrast:border-white px-4 py-2 text-sm hover:bg-zinc-100 dark:hover:bg-zinc-800 theme-high-contrast:hover:bg-zinc-900">
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
              className="rounded-lg bg-zinc-900 dark:bg-zinc-100 text-white dark:text-zinc-900 theme-high-contrast:bg-white theme-high-contrast:text-black px-4 py-2 text-sm font-medium hover:bg-zinc-800 dark:hover:bg-zinc-200 theme-high-contrast:hover:bg-zinc-300"
            >
              Parse JSON
            </button>
            <button
              onClick={() => {
                exportToPdf(findings);
              }}
              disabled={!hasData}
              className="rounded-lg border border-zinc-300 dark:border-zinc-600 theme-high-contrast:border-white px-4 py-2 text-sm disabled:opacity-50 hover:bg-zinc-100 dark:hover:bg-zinc-800 theme-high-contrast:hover:bg-zinc-900"
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

        {hasData && (
          <>
            <section className="grid grid-cols-1 md:grid-cols-2 gap-6">
              <SanctityScore findings={findings} />
              <SummaryChart findings={findings} />
            </section>

            {/* Tab navigation */}
            <div className="flex gap-2 border-b border-zinc-200 dark:border-zinc-700 theme-high-contrast:border-white">
              <button
                onClick={() => setActiveTab("findings")}
                className={`px-4 py-2 text-sm font-medium border-b-2 transition-colors ${activeTab === "findings"
                    ? "border-zinc-900 dark:border-zinc-100 theme-high-contrast:border-yellow-300 text-zinc-900 dark:text-zinc-100 theme-high-contrast:text-yellow-300"
                    : "border-transparent text-zinc-500 hover:text-zinc-700 dark:hover:text-zinc-300 theme-high-contrast:text-white theme-high-contrast:hover:text-yellow-300"
                  }`}
              >
                Findings
              </button>
              <button
                onClick={() => setActiveTab("callgraph")}
                className={`px-4 py-2 text-sm font-medium border-b-2 transition-colors ${activeTab === "callgraph"
                    ? "border-zinc-900 dark:border-zinc-100 theme-high-contrast:border-yellow-300 text-zinc-900 dark:text-zinc-100 theme-high-contrast:text-yellow-300"
                    : "border-transparent text-zinc-500 hover:text-zinc-700 dark:hover:text-zinc-300 theme-high-contrast:text-white theme-high-contrast:hover:text-yellow-300"
                  }`}
              >
                Call Graph
              </button>
            </div>

            {activeTab === "findings" && (
              <>
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

            {activeTab === "callgraph" && (
              <section>
                <CallGraph nodes={callGraphNodes} edges={callGraphEdges} />
              </section>
            )}
          </>
        )}

        {!hasData && !error && (
          <p className="text-center text-zinc-500 dark:text-zinc-400 py-12">
            Load a report to view findings.
          </p>
        )}
      </main>
    </div>
  );
}
