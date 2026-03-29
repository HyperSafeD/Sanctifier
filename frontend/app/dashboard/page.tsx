"use client";

import { useState, useCallback, useTransition, useMemo } from "react";
import dynamic from "next/dynamic";
import type { CallGraphNode, CallGraphEdge, Finding, Severity } from "../types";
import { transformReport, extractCallGraph, normalizeReport } from "../lib/transform";
import { exportToPdf } from "../lib/export-pdf";
import { SeverityFilter } from "../components/SeverityFilter";
import { FindingsList } from "../components/FindingsList";
import { SummaryChart } from "../components/SummaryChart";
import { SanctityScore } from "../components/SanctityScore";
import { ErrorBoundary } from "../components/ErrorBoundary";

const CallGraph = dynamic(() => import("../components/CallGraph").then((m) => m.CallGraph), {
  ssr: false,
  loading: () => (
    <div className="rounded-lg border border-zinc-200 dark:border-zinc-700 bg-white dark:bg-zinc-900 p-6 text-center text-zinc-500">
      Loading call graph…
    </div>
  ),
});

const SAMPLE_JSON = `{
  "size_warnings": [],
  "unsafe_patterns": [],
  "auth_gaps": [],
  "panic_issues": [],
  "arithmetic_issues": []
}`;

type Tab = "findings" | "callgraph";

function extractErrorMessage(payload: unknown, fallback: string): string {
  if (typeof payload === "string" && payload.trim()) {
    return payload;
  }

  if (
    typeof payload === "object" &&
    payload !== null &&
    "error" in payload &&
    typeof payload.error === "string"
  ) {
    return payload.error;
  }

  return fallback;
}

export default function DashboardPage() {
  const [findings, setFindings] = useState<Finding[]>([]);
  const [callGraphNodes, setCallGraphNodes] = useState<CallGraphNode[]>([]);
  const [callGraphEdges, setCallGraphEdges] = useState<CallGraphEdge[]>([]);
  const [severityFilter, setSeverityFilter] = useState<Severity | "all">("all");
  const [error, setError] = useState<string | null>(null);
  const [jsonInput, setJsonInput] = useState("");
  const [activeTab, setActiveTab] = useState<Tab>("findings");
  const [uploadStatus, setUploadStatus] = useState<string | null>(null);
  const [isUploadingContract, setIsUploadingContract] = useState(false);
  const [isPending, startTransition] = useTransition();

  const applyReport = useCallback((rawReport: unknown) => {
    startTransition(() => {
      const report = normalizeReport(rawReport);
      const transformed = transformReport(report);
      const graph = extractCallGraph(report);

      setFindings(transformed);
      setCallGraphNodes(graph.nodes);
      setCallGraphEdges(graph.edges);
    });
  }, []);

  const parseReport = useCallback((text: string) => {
    setError(null);
    setUploadStatus(null);
    try {
      applyReport(JSON.parse(text || SAMPLE_JSON));
    } catch (e) {
      setError(e instanceof Error ? e.message : "Invalid JSON");
      setFindings([]);
      setCallGraphNodes([]);
      setCallGraphEdges([]);
    }
  }, [applyReport]);

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

  const handleContractUpload = useCallback(async (e: React.ChangeEvent<HTMLInputElement>) => {
    const input = e.currentTarget;
    const file = input.files?.[0];
    input.value = "";

    if (!file) {
      return;
    }

    setError(null);
    setUploadStatus(`Analyzing ${file.name}...`);
    setIsUploadingContract(true);

    try {
      const formData = new FormData();
      formData.append("contract", file);

      const response = await fetch("/api/analyze", {
        method: "POST",
        body: formData,
      });
      const rawBody = await response.text();

      let payload: unknown = null;
      if (rawBody) {
        try {
          payload = JSON.parse(rawBody);
        } catch {
          payload = rawBody;
        }
      }

      if (!response.ok) {
        throw new Error(extractErrorMessage(payload, "Contract analysis failed"));
      }

      setJsonInput(JSON.stringify(payload, null, 2));
      applyReport(payload);
      setUploadStatus(`Analysis report ready for ${file.name}.`);
    } catch (uploadError) {
      setUploadStatus(null);
      setError(
        uploadError instanceof Error ? uploadError.message : "Contract analysis failed"
      );
    } finally {
      setIsUploadingContract(false);
    }
  }, [applyReport]);

  const hasData = findings.length > 0 || callGraphNodes.length > 0 || callGraphEdges.length > 0;
  const isProcessing = isPending || isUploadingContract;
  const hasLoadedReport = jsonInput.trim().length > 0;

  return (
    <div className="min-h-screen bg-zinc-50 dark:bg-zinc-950 text-zinc-900 dark:text-zinc-100 theme-high-contrast:bg-black theme-high-contrast:text-white">
      <main className="max-w-6xl mx-auto px-4 sm:px-6 py-8 space-y-8">
        <section className="rounded-xl border border-zinc-200 dark:border-zinc-800 theme-high-contrast:border-white bg-white dark:bg-zinc-900 theme-high-contrast:bg-black p-6 shadow-sm">
          <div className="flex items-center justify-between mb-4">
            <h2 className="text-lg font-semibold theme-high-contrast:text-yellow-300">Load Analysis Report</h2>
            <Link 
              href="/dashboard/webhooks" 
              className="flex items-center gap-2 text-xs font-bold text-zinc-500 hover:text-emerald-500 transition-colors bg-zinc-50 dark:bg-zinc-950 px-3 py-1.5 rounded-lg border border-zinc-200 dark:border-zinc-800"
            >
              <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><path d="M10 13a5 5 0 0 0 7.54.54l3-3a5 5 0 0 0-7.07-7.07l-1.72 1.71"/><path d="M14 11a5 5 0 0 0-7.54-.54l-3 3a5 5 0 0 0 7.07 7.07l1.71-1.71"/></svg>
              Manage Webhooks
            </Link>
          </div>
          <p className="text-sm text-zinc-600 dark:text-zinc-400 theme-high-contrast:text-white mb-4">
            Paste JSON from <code className="bg-zinc-100 dark:bg-zinc-800 theme-high-contrast:bg-zinc-900 px-1 rounded">sanctifier analyze --format json</code>, upload an existing report, or analyze a Rust contract source file.
          </p>
          <div className="flex flex-wrap gap-2 sm:gap-4">
            <label className="flex-1 sm:flex-none text-center cursor-pointer rounded-lg border border-zinc-300 dark:border-zinc-600 theme-high-contrast:border-white px-4 py-2 text-sm hover:bg-zinc-100 dark:hover:bg-zinc-800 theme-high-contrast:hover:bg-zinc-900 focus-within:outline-none focus-within:ring-2 focus-within:ring-zinc-400 focus-within:ring-offset-2">
              Upload JSON
              <input
                type="file"
                accept=".json"
                className="hidden"
                aria-label="JSON report file"
                data-testid="json-upload-input"
                onChange={handleFileUpload}
              />
            </label>
            <label className="flex-1 sm:flex-none text-center cursor-pointer rounded-lg border border-zinc-300 dark:border-zinc-600 theme-high-contrast:border-white px-4 py-2 text-sm hover:bg-zinc-100 dark:hover:bg-zinc-800 theme-high-contrast:hover:bg-zinc-900 focus-within:outline-none focus-within:ring-2 focus-within:ring-zinc-400 focus-within:ring-offset-2">
              {isProcessing ? "Processing..." : "Upload Contract"}
              <input
                type="file"
                accept=".rs"
                className="hidden"
                aria-label="Contract file"
                data-testid="contract-upload-input"
                onChange={handleContractUpload}
              />
            </label>
            <button
              onClick={loadReport}
              className="flex-1 sm:flex-none rounded-lg bg-zinc-900 dark:bg-zinc-100 text-white dark:text-zinc-900 theme-high-contrast:bg-white theme-high-contrast:text-black px-4 py-2 text-sm font-medium hover:bg-zinc-800 dark:hover:bg-zinc-200 theme-high-contrast:hover:bg-zinc-300 focus:outline-none focus-visible:ring-2 focus-visible:ring-zinc-400 focus-visible:ring-offset-2"
            >
              Parse JSON
            </button>
            <button
              onClick={() => {
                exportToPdf(findings);
              }}
              disabled={!hasData}
              className="flex-1 sm:flex-none rounded-lg border border-zinc-300 dark:border-zinc-600 theme-high-contrast:border-white px-4 py-2 text-sm disabled:opacity-50 hover:bg-zinc-100 dark:hover:bg-zinc-800 theme-high-contrast:hover:bg-zinc-900 focus:outline-none focus-visible:ring-2 focus-visible:ring-zinc-400 focus-visible:ring-offset-2 disabled:focus-visible:ring-0"
            >
              Export PDF
            </button>
          </div>
          {uploadStatus && (
            <p className="mt-2 text-sm text-emerald-600 dark:text-emerald-400" role="status" aria-live="polite">
              {uploadStatus}
            </p>
          )}
          {error && (
            <p className="mt-2 text-sm text-red-600 dark:text-red-400">{error}</p>
          )}
          <textarea
            value={jsonInput}
            onChange={(e) => setJsonInput(e.target.value)}
            placeholder={SAMPLE_JSON}
            disabled={isProcessing}
            className="mt-4 w-full h-32 rounded-lg border border-zinc-300 dark:border-zinc-600 bg-white dark:bg-zinc-950 p-3 font-mono text-sm focus:ring-2 focus:ring-zinc-400 dark:focus:ring-zinc-600 outline-none disabled:opacity-50"
          />
        </section>

        {hasData && (
          <>
            <section className="grid grid-cols-1 md:grid-cols-2 gap-6">
              <ErrorBoundary>
                <SanctityScore findings={findings} />
              </ErrorBoundary>
              <ErrorBoundary>
                <SummaryChart findings={findings} />
              </ErrorBoundary>
            </section>

            {/* Tab navigation */}
            <div className="flex gap-2 border-b border-zinc-200 dark:border-zinc-700 theme-high-contrast:border-white" role="tablist" aria-label="Analysis view tabs">
              <button
                onClick={() => setActiveTab("findings")}
                role="tab"
                aria-selected={activeTab === "findings"}
                aria-controls="findings-panel"
                id="findings-tab"
                className={`px-4 py-2 text-sm font-medium border-b-2 transition-colors focus:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-zinc-400 ${activeTab === "findings"
                    ? "border-zinc-900 dark:border-zinc-100 theme-high-contrast:border-yellow-300 text-zinc-900 dark:text-zinc-100 theme-high-contrast:text-yellow-300"
                    : "border-transparent text-zinc-500 hover:text-zinc-700 dark:hover:text-zinc-300 theme-high-contrast:text-white theme-high-contrast:hover:text-yellow-300"
                  }`}
              >
                Findings
              </button>
              <button
                onClick={() => setActiveTab("callgraph")}
                role="tab"
                aria-selected={activeTab === "callgraph"}
                aria-controls="callgraph-panel"
                id="callgraph-tab"
                className={`px-4 py-2 text-sm font-medium border-b-2 transition-colors focus:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-zinc-400 ${activeTab === "callgraph"
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

                <section id="findings-panel" role="tabpanel" aria-labelledby="findings-tab">
                  <h2 className="text-lg font-semibold mb-4">Findings</h2>
                  <ErrorBoundary>
                    <FindingsList findings={findings} severityFilter={severityFilter} />
                  </ErrorBoundary>
                </section>
              </>
            )}

            {activeTab === "callgraph" && (
              <section id="callgraph-panel" role="tabpanel" aria-labelledby="callgraph-tab">
                <ErrorBoundary>
                  <CallGraph nodes={callGraphNodes} edges={callGraphEdges} />
                </ErrorBoundary>
              </section>
            )}
          </>
        )}

        {!hasData && !error && !hasLoadedReport && (
          <p className="text-center text-zinc-500 dark:text-zinc-400 py-12">
            Load a report to view findings.
          </p>
        )}

        {!hasData && !error && hasLoadedReport && (
          <p className="text-center text-zinc-500 dark:text-zinc-400 py-12">
            No findings were detected in the loaded report.
          </p>
        )}
      </main>
    </div>
  );
}
