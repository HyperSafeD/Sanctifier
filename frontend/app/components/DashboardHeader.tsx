"use client";

import Link from "next/link";
import React, { useState } from "react";
import type { RejectedFile } from "../lib/upload-validation";

export type FileProgress = "pending" | "analyzing" | "done" | "error";

interface DashboardHeaderProps {
  jsonInput: string;
  setJsonInput: (v: string) => void;
  loadReport: () => void;
  handleFileUpload: (e: React.ChangeEvent<HTMLInputElement>) => void;
  onContractFiles: (files: File[]) => void;
  exportToPdf: () => void;
  hasData: boolean;
  isProcessing: boolean;
  uploadStatus: string | null;
  error: string | null;
  sampleJson: string;
  batchProgress?: Record<string, FileProgress>;
  rejectedFiles?: RejectedFile[];
}

const PROGRESS_ICON: Record<FileProgress, string> = {
  pending:   "○",
  analyzing: "…",
  done:      "✓",
  error:     "✗",
};

export function DashboardHeader({
  jsonInput,
  setJsonInput,
  loadReport,
  handleFileUpload,
  onContractFiles,
  exportToPdf,
  hasData,
  isProcessing,
  uploadStatus,
  error,
  sampleJson,
  batchProgress,
  rejectedFiles,
}: DashboardHeaderProps) {
  const [isDragging, setIsDragging] = useState(false);

  const handleDragOver = (e: React.DragEvent) => {
    e.preventDefault();
    setIsDragging(true);
  };

  const handleDragLeave = (e: React.DragEvent) => {
    // Only clear when leaving the zone itself, not child elements
    if (!e.currentTarget.contains(e.relatedTarget as Node)) {
      setIsDragging(false);
    }
  };

  const handleDrop = (e: React.DragEvent) => {
    e.preventDefault();
    setIsDragging(false);
    const files = Array.from(e.dataTransfer.files);
    if (files.length > 0) onContractFiles(files);
  };

  const batchEntries = batchProgress ? Object.entries(batchProgress) : [];
  const showBatchList = batchEntries.length > 1;

  return (
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
          {isProcessing ? "Processing…" : "Upload Contract"}
          <input
            type="file"
            accept=".rs"
            multiple
            className="hidden"
            aria-label="Contract file"
            data-testid="contract-upload-input"
            onChange={(e) => {
              const files = Array.from(e.target.files ?? []);
              if (files.length > 0) onContractFiles(files);
              e.target.value = "";
            }}
          />
        </label>
        <button
          onClick={loadReport}
          className="flex-1 sm:flex-none rounded-lg bg-zinc-900 dark:bg-zinc-100 text-white dark:text-zinc-900 theme-high-contrast:bg-white theme-high-contrast:text-black px-4 py-2 text-sm font-medium hover:bg-zinc-800 dark:hover:bg-zinc-200 theme-high-contrast:hover:bg-zinc-300 focus:outline-none focus-visible:ring-2 focus-visible:ring-zinc-400 focus-visible:ring-offset-2"
        >
          Parse JSON
        </button>
        <button
          onClick={exportToPdf}
          disabled={!hasData}
          className="flex-1 sm:flex-none rounded-lg border border-zinc-300 dark:border-zinc-600 theme-high-contrast:border-white px-4 py-2 text-sm disabled:opacity-50 hover:bg-zinc-100 dark:hover:bg-zinc-800 theme-high-contrast:hover:bg-zinc-900 focus:outline-none focus-visible:ring-2 focus-visible:ring-zinc-400 focus-visible:ring-offset-2 disabled:focus-visible:ring-0"
        >
          Export PDF
        </button>
      </div>

      {/* Drag-and-drop batch upload zone */}
      <div
        onDragOver={handleDragOver}
        onDragLeave={handleDragLeave}
        onDrop={handleDrop}
        className={`mt-3 rounded-lg border-2 border-dashed px-4 py-5 text-center text-sm transition-colors select-none ${
          isDragging
            ? "border-zinc-600 dark:border-zinc-300 bg-zinc-50 dark:bg-zinc-800 text-zinc-700 dark:text-zinc-200"
            : "border-zinc-300 dark:border-zinc-700 text-zinc-400 dark:text-zinc-500"
        }`}
        aria-label="Drop zone for .rs contract files"
      >
        {isDragging
          ? "Drop .rs files to analyze"
          : "Drag & drop one or more .rs contract files here for batch analysis"}
      </div>

      {uploadStatus && (
        <p className="mt-2 text-sm text-emerald-600 dark:text-emerald-400" role="status" aria-live="polite">
          {uploadStatus}
        </p>
      )}
      {error && (
        <p className="mt-2 text-sm text-red-600 dark:text-red-400">{error}</p>
      )}

      {/* Rejected files toast */}
      {rejectedFiles && rejectedFiles.length > 0 && (
        <div
          role="alert"
          className="mt-2 rounded-lg bg-amber-50 dark:bg-amber-900/20 border border-amber-200 dark:border-amber-700 px-3 py-2 text-xs text-amber-800 dark:text-amber-300"
        >
          <p className="font-medium mb-0.5">Skipped {rejectedFiles.length} file{rejectedFiles.length > 1 ? "s" : ""}:</p>
          <ul className="space-y-0.5 list-disc list-inside">
            {rejectedFiles.map(({ name, reason }) => (
              <li key={name}>
                <span className="font-mono">{name}</span> — {reason}
              </li>
            ))}
          </ul>
        </div>
      )}

      {/* Per-file batch progress list */}
      {showBatchList && (
        <ul className="mt-3 space-y-1" aria-label="Batch analysis progress">
          {batchEntries.map(([name, status]) => (
            <li key={name} className="flex items-center gap-2 text-xs font-mono">
              <span
                className={
                  status === "done" ? "text-emerald-600 dark:text-emerald-400"
                  : status === "error" ? "text-red-600 dark:text-red-400"
                  : "text-zinc-400 dark:text-zinc-500"
                }
              >
                {PROGRESS_ICON[status]}
              </span>
              <span className="text-zinc-700 dark:text-zinc-300 truncate max-w-xs">{name}</span>
              <span className="text-zinc-400 dark:text-zinc-500 shrink-0">
                {status === "analyzing" && "Analyzing…"}
                {status === "done" && "Done"}
                {status === "error" && "Failed"}
              </span>
            </li>
          ))}
        </ul>
      )}

      <textarea
        value={jsonInput}
        onChange={(e) => setJsonInput(e.target.value)}
        placeholder={sampleJson}
        disabled={isProcessing}
        className="mt-4 w-full h-32 rounded-lg border border-zinc-300 dark:border-zinc-600 bg-white dark:bg-zinc-950 p-3 font-mono text-sm focus:ring-2 focus:ring-zinc-400 dark:focus:ring-zinc-600 outline-none disabled:opacity-50"
      />
    </section>
  );
}
