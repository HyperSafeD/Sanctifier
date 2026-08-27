"use client";

import { useCallback } from "react";
import dynamic from "next/dynamic";
import { AnalysisTerminal } from "../components/AnalysisTerminal";
import { SanctityScore } from "../components/SanctityScore";
import { FindingsList } from "../components/FindingsList";
import { ZkFindingsPanel } from "../components/ZkFindingsPanel";
import { SeverityFilter } from "../components/SeverityFilter";
import { ErrorBoundary } from "../components/ErrorBoundary";
import { CallGraphSkeleton } from "../components/CallGraphSkeleton";
import { nextScanProgressPhase } from "../lib/scan-progress";
import { getSettingsHeaders } from "../lib/settings";
import { ScanProvider, useScan } from "./ScanContext";
import Link from "next/link";

const CallGraph = dynamic(() => import("../components/CallGraph").then((m) => m.CallGraph), {
  ssr: false,
  loading: () => (
    <div 
      className="h-[400px] w-full flex items-center justify-center rounded-xl border border-zinc-200 dark:border-zinc-800 bg-white dark:bg-zinc-900 text-zinc-500"
      role="status"
      aria-label="Loading call graph visualization"
    >
      <span>Loading call graph…</span>
    </div>
  ),
  loading: () => <CallGraphSkeleton />,
});

function ScanPageContent() {
  const {
    logs,
    isAnalyzing,
    findings,
    error,
    selectedFile,
    severityFilter,
    addLog,
    setLogs,
    setIsAnalyzing,
    setFindings,
    setError,
    setSelectedFile,
    setSeverityFilter,
    resetScan,
  } = useScan();

  const handleFileChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (file) {
      setSelectedFile(file);
      resetScan();
    }
  };

  const runAnalysis = useCallback(async () => {
    if (!selectedFile) return;

    setIsAnalyzing(true);
    setError(null);
    setFindings([]);
    setLogs([]);

    addLog(`Starting analysis for ${selectedFile.name}...`);
    addLog(`Uploading contract to analysis engine...`);

    try {
      const formData = new FormData();
      formData.append("contract", selectedFile);

      // We start a "simulated" log stream since our POST is atomic
      let phaseIndex = 0;
      const logsTimer = setInterval(() => {
        const phase = nextScanProgressPhase(phaseIndex);
        phaseIndex += 1;
        setLogs((prev) => [...prev, `[${new Date().toLocaleTimeString()}] [INFO] ${phase}`]);
      }, 1500);

      const response = await fetch("/api/analyze", {
        method: "POST",
        body: formData,
        headers: getSettingsHeaders() as Record<string, string>,
      });

      clearInterval(logsTimer);

      const data = await response.json();

      if (!response.ok) {
        throw new Error(data.error || "Analysis failed");
      }

      setFindings(data);
      addLog(`Analysis complete. Found ${data.length} potential issues.`);
      addLog(`SUCCESS: Security report generated.`);
    } catch (err) {
      const msg = err instanceof Error ? err.message : "Analysis failed";
      setError(msg);
      addLog(`ERROR: ${msg}`);
    } finally {
      setIsAnalyzing(false);
    }
  }, [selectedFile, addLog, setIsAnalyzing, setError, setFindings, setLogs]);

  return (
    <div className="min-h-screen bg-zinc-50 dark:bg-zinc-950 text-zinc-900 dark:text-zinc-100 pb-20">
      <main className="max-w-6xl mx-auto px-4 sm:px-6 py-12 space-y-12">
        {/* Header */}
        <header className="space-y-4 text-center">
          <h1 className="text-4xl font-bold tracking-tight sm:text-5xl bg-gradient-to-r from-zinc-900 to-zinc-500 dark:from-zinc-50 dark:to-zinc-500 bg-clip-text text-transparent">
            Security Scanner
          </h1>
          <p className="text-lg text-zinc-600 dark:text-zinc-400 max-w-2xl mx-auto">
            Upload your Soroban contract source file (.rs) for an instant deep-dive security audit.
          </p>
        </header>

        {/* Upload & Controls */}
        <section aria-label="Contract upload and analysis controls" className="flex flex-col items-center gap-8">
          <div className="w-full max-w-2xl group relative">
            <div className={`absolute -inset-1 bg-gradient-to-r from-emerald-500 to-blue-500 rounded-2xl blur opacity-20 group-hover:opacity-40 transition duration-1000 ${isAnalyzing ? "animate-pulse" : ""}`} />
            <label className={`relative block overflow-hidden rounded-2xl border-2 border-dashed transition-all cursor-pointer bg-white dark:bg-zinc-900 shadow-xl ${selectedFile
              ? "border-emerald-500/50 bg-emerald-500/5"
              : "border-zinc-200 dark:border-zinc-800 hover:border-zinc-300 dark:hover:border-zinc-700"
              }`}>
              <input
                type="file"
                accept=".rs"
                onChange={handleFileChange}
                className="hidden"
                disabled={isAnalyzing}
                aria-label="Upload Rust contract file"
                aria-describedby="file-upload-description"
              />
              <div className="px-8 py-12 flex flex-col items-center text-center space-y-4">
                <div 
                  className={`w-16 h-16 rounded-2xl flex items-center justify-center transition-colors ${selectedFile ? "bg-emerald-500/10 text-emerald-500" : "bg-zinc-100 dark:bg-zinc-800 text-zinc-400"}`}
                  aria-hidden="true"
                >
                  <svg xmlns="http://www.w3.org/2000/svg" width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" /><polyline points="17 8 12 3 7 8" /><line x1="12" y1="3" x2="12" y2="15" /></svg>
                </div>
                <div>
                  <p className="text-lg font-bold">
                    {selectedFile ? selectedFile.name : "Choose a Rust contract"}
                  </p>
                  <p id="file-upload-description" className="text-sm text-zinc-500">
                    Click to browse or drag and drop your .rs file
                  </p>
                </div>
              </div>
            </label>
          </div>

          <div className="flex gap-4">
            <button
              onClick={runAnalysis}
              disabled={!selectedFile || isAnalyzing}
              aria-label={isAnalyzing ? "Analysis in progress" : "Run security audit on selected contract"}
              aria-disabled={!selectedFile || isAnalyzing}
              className={`px-10 py-4 rounded-2xl font-bold transition-all shadow-2xl active:scale-95 flex items-center gap-3 ${!selectedFile || isAnalyzing
                ? "bg-zinc-200 dark:bg-zinc-800 text-zinc-400 cursor-not-allowed"
                : "bg-zinc-900 dark:bg-zinc-100 text-white dark:text-zinc-900 hover:bg-zinc-800 dark:hover:bg-zinc-200 hover:scale-105 shadow-emerald-500/20"
                }`}
            >
              {isAnalyzing ? (
                <>
                  <div className="w-5 h-5 border-2 border-zinc-400 border-t-transparent rounded-full animate-spin" role="status" aria-label="Loading" />
                  Running Audit...
                </>
              ) : (
                "Run Security Audit"
              )}
            </button>
          </div>
        </section>

        {/* Console / Terminal Section */}
        {(logs.length > 0 || isAnalyzing) && (
          <section 
            className="space-y-4 animate-in fade-in slide-in-from-bottom-4 duration-500"
            aria-label="Analysis progress logs"
            role="region"
          >
            <div className="flex items-center justify-between">
              <h2 className="text-xl font-bold flex items-center gap-2">
                <div className="w-2 h-2 rounded-full bg-emerald-500 animate-pulse" aria-hidden="true" />
                Live Analysis Stream
              </h2>
              <span className="text-xs font-mono text-zinc-500" aria-label="CLI Emulator version">CLI_EMULATOR_v1.0</span>
            </div>
            <AnalysisTerminal logs={logs} isAnalyzing={isAnalyzing} />
          </section>
        )}

        {/* Error State */}
        {error && (
          <section 
            className="p-6 rounded-2xl border border-red-200 dark:border-red-900/50 bg-red-50 dark:bg-red-900/10 text-red-600 dark:text-red-400 flex flex-col items-center gap-4 text-center animate-in zoom-in-95 duration-300"
            role="alert"
            aria-live="assertive"
          >
            <svg xmlns="http://www.w3.org/2000/svg" width="40" height="40" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true"><circle cx="12" cy="12" r="10" /><line x1="12" y1="8" x2="12" y2="12" /><line x1="12" y1="16" x2="12.01" y2="16" /></svg>
            <div className="space-y-1">
              <h3 className="font-bold text-lg">Analysis Failed</h3>
              <p className="max-w-md">{error}</p>
            </div>
            <button 
              onClick={runAnalysis} 
              className="text-sm font-bold underline underline-offset-4 hover:opacity-80 transition-opacity"
              aria-label="Retry security analysis"
            >
              Try Again
            </button>
          </section>
        )}

        {/* Results Section */}
        {findings.length > 0 && !isAnalyzing && (
          <section 
            className="space-y-12 animate-in fade-in duration-1000 pt-10 border-t border-zinc-200 dark:border-zinc-800"
            aria-label="Analysis results"
            role="region"
          >
            <div className="grid grid-cols-1 md:grid-cols-2 gap-8">
              <ErrorBoundary compact>
                <SanctityScore findings={findings} />
              </ErrorBoundary>
              <div className="space-y-6">
                <div className="p-8 rounded-3xl border border-zinc-200 dark:border-zinc-800 bg-white dark:bg-zinc-900 shadow-sm flex flex-col justify-between h-full">
                  <div className="space-y-4">
                    <h3 className="text-2xl font-bold">Analysis Summary</h3>
                    <p className="text-zinc-500">
                      The automated engine has completed its sweep of your contract.
                      You can view the detailed findings below or explore the full reporting dashboard.
                    </p>
                  </div>
                  <div className="flex flex-col sm:flex-row items-center gap-4 mt-8">
                    <Link 
                      href="/dashboard" 
                      className="inline-flex items-center gap-2 text-emerald-500 font-bold hover:gap-3 transition-all"
                      aria-label="Navigate to full dashboard"
                    >
                      Open Full Dashboard
                      <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true"><path d="M5 12h14" /><path d="m12 5 7 7-7 7" /></svg>
                    </Link>
                    <button
                      onClick={() => {
                        const reportId = Math.random().toString(36).substring(7);
                        const shareUrl = `${window.location.origin}/share/${reportId}`;
                        navigator.clipboard.writeText(shareUrl);
                        alert(`Shareable link copied to clipboard: ${shareUrl}\n(Note: In a real system, this ID would be stored in the database with an expiry)`);
                      }}
                      className="inline-flex items-center gap-2 text-zinc-500 hover:text-zinc-900 dark:hover:text-zinc-100 font-medium transition-colors"
                      aria-label="Copy shareable report link to clipboard"
                    >
                      <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true"><path d="M4 12v8a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2v-8" /><polyline points="16 6 12 2 8 6" /><line x1="12" y1="2" x2="12" y2="15" /></svg>
                      Share Report
                    </button>
                  </div>
                </div>
              </div>
            </div>

            <div className="space-y-6">
              <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4">
                <h2 className="text-2xl font-bold tracking-tight">Security Findings</h2>
                <SeverityFilter selected={severityFilter} onChange={setSeverityFilter} />
              </div>
              <ErrorBoundary compact>
                <ZkFindingsPanel findings={findings} />
                <FindingsList findings={findings} severityFilter={severityFilter} />
              </ErrorBoundary>
            </div>

            <div className="space-y-6 pt-10 border-t border-zinc-200 dark:border-zinc-800">
              <h2 className="text-2xl font-bold tracking-tight">System Integrity Map</h2>
              <ErrorBoundary compact>
                <CallGraph nodes={[]} edges={[]} /> {/* Call graph would need more data from API if desired */}
                <p className="text-xs text-zinc-500 text-center italic mt-4">Note: Visualizing complex call structures requires multiple analysis passes.</p>
              </ErrorBoundary>
            </div>
          </section>
        )}
      </main>

    </div>
  );
}

export default function ScanPage() {
  return (
    <ScanProvider>
      <ScanPageContent />
    </ScanProvider>
  );
}
