"use client";

import { useState, useCallback, useMemo, useTransition, useEffect } from "react";
import dynamic from "next/dynamic";
import type { Severity } from "../types";
import { transformReport, extractCallGraph, normalizeReport } from "../lib/transform";
import { normalizeFindingCodeQuery, validateFindingCodeQuery } from "../lib/finding-filters";
import { validateContractBatch } from "../lib/upload-validation";
import type { RejectedFile } from "../lib/upload-validation";
import type { FileProgress } from "../components/DashboardHeader";
import type { WorkspaceSummary, AnalysisReport, Finding } from "../types";
import {
  createWorkspaceFromSingleReport,
  extractErrorMessage,
  isWorkspaceSummary,
  parseJsonInput,
  SAMPLE_JSON,
} from "../lib/report-ingestion";
import { exportToPdf } from "../lib/export-pdf";
import { copyShareLink, isShareLinkTooLarge } from "../lib/share-link";
import { SeverityFilter } from "../components/SeverityFilter";
import { FindingsList } from "../components/FindingsList";
import { SummaryChart } from "../components/SummaryChart";
import { SanctityScore } from "../components/SanctityScore";
import { ComparisonView } from "../components/ComparisonView";
import { ErrorBoundary } from "../components/ErrorBoundary";
import { useWorkspace } from "../providers/WorkspaceProvider";
import { WorkspaceSidebar } from "../components/WorkspaceSidebar";
import { DashboardHeader } from "../components/DashboardHeader";
import { getSettingsHeaders } from "../lib/settings";
import { saveScanRecord, getScanHistory, clearScanHistory, type ScanRecord } from "../lib/scan-history";

const CallGraph = dynamic(() => import("../components/CallGraph").then((m) => m.CallGraph), {
  ssr: false,
  loading: () => (
    <div className="rounded-lg border border-zinc-200 dark:border-zinc-700 bg-white dark:bg-zinc-900 p-6 text-center text-zinc-500">
      Loading call graph…
    </div>
  ),
});

type Tab = "findings" | "callgraph" | "diff";

export default function DashboardPage() {
  const { workspace, selectedContract, setWorkspace, updateContractReport } = useWorkspace();
  const [severityFilter, setSeverityFilter] = useState<Severity | "all">("all");
  const [error, setError] = useState<string | null>(null);
  const [jsonInput, setJsonInput] = useState("");
  const [baselineJsonInput, setBaselineJsonInput] = useState("");
  const [activeTab, setActiveTab] = useState<Tab>("findings");
  const [uploadStatus, setUploadStatus] = useState<string | null>(null);
  const [isUploadingContract, setIsUploadingContract] = useState(false);
  const [codeFilterInput, setCodeFilterInput] = useState("");
  const [codeFilterError, setCodeFilterError] = useState<string | null>(null);
  const [isPending, startTransition] = useTransition();
  const [sidebarOpen, setSidebarOpen] = useState(false);
  const [batchProgress, setBatchProgress] = useState<Record<string, FileProgress>>({});
  const [rejectedFiles, setRejectedFiles] = useState<RejectedFile[]>([]);
  const [trendRecords, setTrendRecords] = useState<ScanRecord[]>([]);
  const [sourceCache, setSourceCache] = useState<Record<string, string>>({});

  const currentReport = selectedContract?.report;
  const currentContractName = selectedContract?.name ?? "default";

  // Load trend data on mount and when workspace changes
  useEffect(() => {
    const wsName = workspace?.workspace ?? currentContractName;
    getScanHistory(wsName).then(setTrendRecords).catch(() => {});
  }, [workspace, currentContractName]);

  const { findings, nodes: callGraphNodes, edges: callGraphEdges } = useMemo(() => {
    if (!currentReport) {
      return {
        findings: [] as ReturnType<typeof transformReport>,
        nodes: [] as ReturnType<typeof extractCallGraph>["nodes"],
        edges: [] as ReturnType<typeof extractCallGraph>["edges"],
      };
    }
    const report = normalizeReport(currentReport);
    return {
      findings: transformReport(report),
      ...extractCallGraph(report)
    };
  }, [currentReport]);

  const baselineReport: AnalysisReport | null = useMemo(() => {
    if (!baselineJsonInput.trim()) return null;
    try {
      const parsed = JSON.parse(baselineJsonInput);
      return normalizeReport(parsed);
    } catch {
      return null;
    }
  }, [baselineJsonInput]);

  const applyReport = useCallback((rawReport: unknown, source?: string) => {
    startTransition(() => {
      if (isWorkspaceSummary(rawReport)) {
        setWorkspace(rawReport);
      } else {
        const ws = createWorkspaceFromSingleReport(rawReport);
        if (source && ws.contracts.length > 0) {
          ws.contracts[0].source = source;
        }
        setWorkspace(ws);
      }
    });
  }, [setWorkspace]);

  const persistScanRecord = useCallback((findingsList: Finding[]) => {
    const wsName = workspace?.workspace ?? currentContractName;
    const counts = { critical: 0, high: 0, medium: 0, low: 0 };
    findingsList.forEach((f) => {
      if (counts[f.severity] !== undefined) counts[f.severity]++;
    });
    saveScanRecord({
      workspace: wsName,
      timestamp: Date.now(),
      ...counts,
      total: findingsList.length,
    }).then(() => {
      getScanHistory(wsName).then(setTrendRecords).catch(() => {});
    }).catch(() => {});
  }, [workspace, currentContractName]);

  const parseReport = useCallback((text: string) => {
    setError(null);
    setUploadStatus(null);
    try {
      const parsed = parseJsonInput(text);
      applyReport(parsed);
      const report = normalizeReport(parsed);
      const findingsList = transformReport(report);
      persistScanRecord(findingsList);
    } catch (e) {
      setError("Invalid JSON");
      setWorkspace(null);
    }
  }, [applyReport, persistScanRecord, setWorkspace]);

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

  const analyzeFile = useCallback(async (file: File): Promise<{ payload: unknown; source: string }> => {
    const formData = new FormData();
    formData.append("contract", file);
    const settingsHeaders = getSettingsHeaders();
    const response = await fetch("/api/analyze", {
      method: "POST",
      body: formData,
      headers: settingsHeaders as Record<string, string>,
    });
    const rawBody = await response.text();
    let payload: unknown = null;
    if (rawBody) {
      try { payload = JSON.parse(rawBody); } catch { payload = rawBody; }
    }
    if (!response.ok) throw new Error(extractErrorMessage(payload, "Contract analysis failed"));
    // Read the source from the File object
    const source = await file.text();
    return { payload, source };
  }, []);

  const getSourceForFinding = useCallback((finding: Finding): string | undefined => {
    const contractName = workspace?.contracts.find(
      (c) => c.source && (finding.location.includes(c.name) || finding.location === c.name)
    )?.name;
    if (contractName && sourceCache[contractName]) {
      return sourceCache[contractName];
    }
    // Fall back to selected contract's source
    return selectedContract?.source;
  }, [workspace, selectedContract, sourceCache]);

  const getFileNameForFinding = useCallback((finding: Finding): string | undefined => {
    return selectedContract?.name ?? "contract.rs";
  }, [selectedContract]);

  const handleContractFiles = useCallback(async (files: File[]) => {
    const { valid, rejected } = validateContractBatch(files);

    if (rejected.length > 0) {
      setRejectedFiles(rejected);
      setTimeout(() => setRejectedFiles([]), 6000);
    }

    if (valid.length === 0) return;

    setError(null);
    setIsUploadingContract(true);

    if (valid.length === 1) {
      const file = valid[0];
      setBatchProgress({ [file.name]: "analyzing" });
      setUploadStatus(`Analyzing ${file.name}…`);
      try {
        const { payload, source } = await analyzeFile(file);
        setSourceCache((prev) => ({ ...prev, [file.name]: source }));
        setJsonInput(JSON.stringify(payload, null, 2));
        applyReport(payload, source);
        // Persist scan record for trend chart
        const report = normalizeReport(payload);
        const transformedFindings = transformReport(report);
        persistScanRecord(transformedFindings);
        setBatchProgress({ [file.name]: "done" });
        setUploadStatus(`Analysis report ready for ${file.name}.`);
      } catch (err) {
        setBatchProgress({ [file.name]: "error" });
        setUploadStatus(null);
        setError(err instanceof Error ? err.message : "Contract analysis failed");
      } finally {
        setIsUploadingContract(false);
      }
      return;
    }

    // Batch: create workspace skeleton, then analyze each file
    const initialProgress: Record<string, FileProgress> = {};
    for (const f of valid) initialProgress[f.name] = "pending";
    setBatchProgress(initialProgress);
    setUploadStatus(`Analyzing ${valid.length} files…`);

    const skeleton: WorkspaceSummary = {
      workspace: "batch-upload",
      contracts: valid.map((f) => ({ name: f.name, total_findings: 0, source: "" })),
      shared_libs: [],
      grand_total_findings: 0,
    };
    setWorkspace(skeleton);

    let doneCount = 0;
    let errorCount = 0;
    const newSourceCache: Record<string, string> = {};

    await Promise.all(
      valid.map(async (file) => {
        setBatchProgress((prev) => ({ ...prev, [file.name]: "analyzing" }));
        try {
          const { payload, source } = await analyzeFile(file);
          newSourceCache[file.name] = source;
          const report = normalizeReport(payload);
          updateContractReport(file.name, report);
          setBatchProgress((prev) => ({ ...prev, [file.name]: "done" }));
          doneCount++;
        } catch {
          setBatchProgress((prev) => ({ ...prev, [file.name]: "error" }));
          errorCount++;
        }
      })
    );

    setSourceCache((prev) => ({ ...prev, ...newSourceCache }));
    setIsUploadingContract(false);
    setUploadStatus(
      `Batch complete: ${doneCount} analyzed${errorCount > 0 ? `, ${errorCount} failed` : ""}.`
    );
  }, [analyzeFile, applyReport, setWorkspace, updateContractReport, persistScanRecord]);

  const handleClearHistory = useCallback(() => {
    const wsName = workspace?.workspace ?? currentContractName;
    clearScanHistory(wsName).then(() => {
      setTrendRecords([]);
    }).catch(() => {});
  }, [workspace, currentContractName]);

  const handleCodeFilterChange = useCallback((input: string) => {
    const normalized = normalizeFindingCodeQuery(input);
    setCodeFilterInput(normalized);
    setCodeFilterError(validateFindingCodeQuery(normalized));
  }, []);

  const handleShareReport = async () => {
    const workspace = selectedContract?.report
      ? { workspace: "sanctifier", contracts: [{ name: selectedContract.name, total_findings: findings.length }], shared_libs: [], grand_total_findings: findings.length }
      : null;
    const data = workspace ?? currentReport;
    if (!data) return;
    if (isShareLinkTooLarge(data as Parameters<typeof copyShareLink>[0])) {
      setError("Report is too large to share via URL. Export as PDF instead.");
      return;
    }
    await copyShareLink(data as Parameters<typeof copyShareLink>[0]);
  };

  const hasData = currentReport !== null;
  const isProcessing = isPending || isUploadingContract;
  const hasLoadedReport = jsonInput.trim().length > 0;

  return (
    <div className="min-h-screen bg-zinc-50 dark:bg-zinc-950 text-zinc-900 dark:text-zinc-100 theme-high-contrast:bg-black theme-high-contrast:text-white">
      <main className="max-w-6xl mx-auto px-4 sm:px-6 py-8 space-y-8">
        <DashboardHeader
          jsonInput={jsonInput}
          setJsonInput={setJsonInput}
          loadReport={loadReport}
          handleFileUpload={handleFileUpload}
          onContractFiles={handleContractFiles}
          exportToPdf={() => exportToPdf(findings)}
          shareReport={handleShareReport}
          hasData={hasData}
          isProcessing={isProcessing}
          uploadStatus={uploadStatus}
          error={error}
          sampleJson={SAMPLE_JSON}
          batchProgress={batchProgress}
          rejectedFiles={rejectedFiles}
        />

        {/* Mobile sidebar hamburger — only shown when a multi-contract workspace is loaded */}
        <div className="md:hidden">
          <button
            aria-label="Open workspace sidebar"
            onClick={() => setSidebarOpen(true)}
            className="flex items-center gap-2 px-3 py-2 rounded-lg border border-zinc-200 dark:border-zinc-700 bg-white dark:bg-zinc-900 text-sm text-zinc-600 dark:text-zinc-400 hover:text-zinc-900 dark:hover:text-zinc-100 transition-colors"
          >
            <svg width="18" height="18" viewBox="0 0 18 18" fill="none" aria-hidden="true">
              <rect y="3" width="18" height="2" rx="1" fill="currentColor" />
              <rect y="8" width="18" height="2" rx="1" fill="currentColor" />
              <rect y="13" width="18" height="2" rx="1" fill="currentColor" />
            </svg>
            Contracts
          </button>
        </div>

        <div className="flex flex-col md:flex-row gap-8">
          <WorkspaceSidebar isOpen={sidebarOpen} onClose={() => setSidebarOpen(false)} />

          <div className="flex-1 space-y-8">
            {hasData && (
              <>
                <section className="grid grid-cols-1 md:grid-cols-2 gap-6">
                  <ErrorBoundary>
                    <SanctityScore findings={findings} />
                  </ErrorBoundary>
                  <ErrorBoundary>
                    <SummaryChart
                      findings={findings}
                      trendRecords={trendRecords}
                      onClearHistory={handleClearHistory}
                    />
                  </ErrorBoundary>
                </section>

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
                  <button
                    onClick={() => setActiveTab("diff")}
                    role="tab"
                    aria-selected={activeTab === "diff"}
                    aria-controls="diff-panel"
                    id="diff-tab"
                    className={`px-4 py-2 text-sm font-medium border-b-2 transition-colors focus:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-zinc-400 ${activeTab === "diff"
                        ? "border-zinc-900 dark:border-zinc-100 theme-high-contrast:border-yellow-300 text-zinc-900 dark:text-zinc-100 theme-high-contrast:text-yellow-300"
                        : "border-transparent text-zinc-500 hover:text-zinc-700 dark:hover:text-zinc-300 theme-high-contrast:text-white theme-high-contrast:hover:text-yellow-300"
                      }`}
                  >
                    Diff
                  </button>
                </div>

                {activeTab === "findings" && (
                  <>
                    <section>
                      <h2 className="text-lg font-semibold mb-4">Filter Findings</h2>
                      <div className="space-y-4">
                        <SeverityFilter selected={severityFilter} onChange={setSeverityFilter} />
                        <div className="max-w-xs">
                          <label htmlFor="finding-code-filter" className="mb-1 block text-sm font-medium">
                            Search by finding code
                          </label>
                          <input
                            id="finding-code-filter"
                            type="text"
                            value={codeFilterInput}
                            onChange={(event) => handleCodeFilterChange(event.target.value)}
                            placeholder="S001"
                            inputMode="text"
                            autoCapitalize="characters"
                            autoComplete="off"
                            spellCheck={false}
                            aria-invalid={Boolean(codeFilterError)}
                            aria-describedby="finding-code-filter-help"
                            className="w-full rounded-lg border border-zinc-300 bg-white px-3 py-2 font-mono text-sm outline-none transition focus-visible:ring-2 focus-visible:ring-zinc-400 dark:border-zinc-600 dark:bg-zinc-950"
                          />
                          <p
                            id="finding-code-filter-help"
                            className={`mt-1 text-xs ${codeFilterError ? "text-red-600 dark:text-red-400" : "text-zinc-500 dark:text-zinc-400"}`}
                          >
                            {codeFilterError ?? "Use exact finding codes like S001, S012, or S020."}
                          </p>
                        </div>
                      </div>
                    </section>

                    <section id="findings-panel" role="tabpanel" aria-labelledby="findings-tab">
                      <h2 className="text-lg font-semibold mb-4">Findings</h2>
                      <ErrorBoundary>
                        <FindingsList
                          findings={findings}
                          severityFilter={severityFilter}
                          codeFilter={codeFilterError ? "" : codeFilterInput}
                          getSource={getSourceForFinding}
                          getFileName={getFileNameForFinding}
                        />
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

                {activeTab === "diff" && (
                  <section id="diff-panel" role="tabpanel" aria-labelledby="diff-tab">
                    <div className="space-y-6">
                      <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                        <div>
                          <label htmlFor="baseline-json-input" className="mb-1 block text-sm font-medium">
                            Baseline Report (JSON)
                          </label>
                          <textarea
                            id="baseline-json-input"
                            value={baselineJsonInput}
                            onChange={(e) => setBaselineJsonInput(e.target.value)}
                            placeholder='Paste baseline JSON report here...'
                            rows={4}
                            className="w-full rounded-lg border border-zinc-300 bg-white px-3 py-2 font-mono text-xs outline-none transition focus-visible:ring-2 focus-visible:ring-zinc-400 dark:border-zinc-600 dark:bg-zinc-950"
                          />
                        </div>
                        <div>
                          <label className="mb-1 block text-sm font-medium">
                            Current Report
                          </label>
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
                          currentReport={currentReport ? normalizeReport(currentReport) : null}
                          baselineName="Baseline"
                          currentName={selectedContract?.name ?? "Current"}
                        />
                      </ErrorBoundary>
                    </div>
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
          </div>
        </div>
      </main>
    </div>
  );
}
