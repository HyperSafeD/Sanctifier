"use client";

import { useState, useCallback } from "react";
import { AnalysisTerminal } from "../components/AnalysisTerminal";

export default function TerminalPage() {
  const [logs, setLogs] = useState<string[]>([]);
  const [isAnalyzing, setIsAnalyzing] = useState(false);
  const [connectionError, setConnectionError] = useState<string | null>(null);

  const startAnalysis = useCallback(() => {
    setLogs([]);
    setIsAnalyzing(true);
    setConnectionError(null);

    const eventSource = new EventSource("/api/analyze?path=.");

    eventSource.onmessage = (event) => {
      const data = JSON.parse(event.data);
      setLogs((prev) => [...prev, data]);

      if (data.includes("Analysis complete")) {
        eventSource.close();
        setIsAnalyzing(false);
      }
    };

    eventSource.onerror = (err) => {
      console.error("EventSource failed:", err);
      setConnectionError("Connection lost or server error. Click 'Reconnect' to try again.");
      eventSource.close();
      setIsAnalyzing(false);
    };

    return () => {
      eventSource.close();
    };
  }, []);

  const handleReconnect = useCallback(() => {
    setConnectionError(null);
    startAnalysis();
  }, [startAnalysis]);

  return (
    <div className="min-h-screen bg-zinc-50 dark:bg-zinc-950 text-zinc-900 dark:text-zinc-100">
      <main className="max-w-5xl mx-auto px-6 py-12 space-y-8">
        <div className="flex flex-col md:flex-row md:items-end justify-between gap-6">
          <div className="space-y-2">
            <h1 className="text-3xl font-bold tracking-tight">Analysis Terminal</h1>
            <p className="text-zinc-600 dark:text-zinc-400 max-w-2xl">
              Monitor your contract&apos;s security analysis in real-time. This interactive terminal
              streams live logs directly from the Sanctifier core engine.
            </p>
          </div>

          <div className="flex gap-3">
            {connectionError && (
              <button
                onClick={handleReconnect}
                className="px-6 py-3 rounded-xl font-bold transition-all shadow-lg hover:shadow-xl active:scale-95 flex items-center gap-2 bg-amber-500 text-white hover:bg-amber-600"
              >
                <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><path d="M21 12a9 9 0 0 0-9-9 9.75 9.75 0 0 0-6.74 2.74L3 8"/><path d="M3 3v5h5"/><path d="M3 12a9 9 0 0 0 9 9 9.75 9.75 0 0 0 6.74-2.74L21 16"/><path d="M16 16h5v5"/></svg>
                Reconnect
              </button>
            )}
            <button
              onClick={startAnalysis}
              disabled={isAnalyzing}
              className={`px-8 py-3 rounded-xl font-bold transition-all shadow-lg hover:shadow-xl active:scale-95 flex items-center gap-2 ${isAnalyzing
                ? "bg-zinc-200 dark:bg-zinc-800 text-zinc-400 cursor-not-allowed"
                : "bg-zinc-900 dark:bg-zinc-100 text-white dark:text-zinc-900 hover:bg-zinc-800 dark:hover:bg-zinc-200"
              }`}
            >
              {isAnalyzing ? (
                <>
                  <div className="w-4 h-4 border-2 border-zinc-400 border-t-transparent rounded-full animate-spin" />
                  Analyzing...
                </>
              ) : (
                "Start New Analysis"
              )}
            </button>
          </div>
        </div>

        {connectionError && (
          <div className="p-4 rounded-xl border border-amber-200 dark:border-amber-800 bg-amber-50 dark:bg-amber-900/20 text-amber-800 dark:text-amber-200 flex items-center gap-3">
            <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><path d="m21.73 18-8-14a2 2 0 0 0-3.48 0l-8 14A2 2 0 0 0 4 21h16a2 2 0 0 0 1.73-3Z"/><line x1="12" y1="9" x2="12" y2="13"/><line x1="12" y1="17" x2="12.01" y2="17"/></svg>
            <span>{connectionError}</span>
          </div>
        )}

        <section className="relative">
          <div className="absolute -inset-1 bg-gradient-to-r from-emerald-500/20 to-blue-500/20 rounded-2xl blur opacity-25 group-hover:opacity-100 transition duration-1000 group-hover:duration-200"></div>
          <AnalysisTerminal logs={logs} isAnalyzing={isAnalyzing} />
        </section>

        <section className="grid grid-cols-1 md:grid-cols-3 gap-6">
          <div className="p-6 rounded-2xl border border-zinc-200 dark:border-zinc-800 bg-white dark:bg-zinc-900 shadow-sm">
            <div className="w-10 h-10 rounded-full bg-emerald-500/10 flex items-center justify-center text-emerald-500 mb-4">
              <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z" /></svg>
            </div>
            <h3 className="font-bold mb-2">Live Scanning</h3>
            <p className="text-sm text-zinc-500 dark:text-zinc-400">Watch as our engine traverses your Soroban contract code in real-time.</p>
          </div>
          <div className="p-6 rounded-2xl border border-zinc-200 dark:border-zinc-800 bg-white dark:bg-zinc-900 shadow-sm">
            <div className="w-10 h-10 rounded-full bg-blue-500/10 flex items-center justify-center text-blue-500 mb-4">
              <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><polyline points="16 18 22 12 16 6" /><polyline points="8 6 2 12 8 18" /></svg>
            </div>
            <h3 className="font-bold mb-2">Instant Feedback</h3>
            <p className="text-sm text-zinc-500 dark:text-zinc-400">Get immediate diagnostic information without waiting for long build processes.</p>
          </div>
          <div className="p-6 rounded-2xl border border-zinc-200 dark:border-zinc-800 bg-white dark:bg-zinc-900 shadow-sm">
            <div className="w-10 h-10 rounded-full bg-amber-500/10 flex items-center justify-center text-amber-500 mb-4">
              <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" /><polyline points="7 10 12 15 17 10" /><line x1="12" y1="15" x2="12" y2="3" /></svg>
            </div>
            <h3 className="font-bold mb-2">Export Logs</h3>
            <p className="text-sm text-zinc-500 dark:text-zinc-400">Keep a record of your analysis sessions for compliance and auditing purposes.</p>
          </div>
        </section>
      </main>
    </div>
  );
}
