"use client";

import { createContext, useContext, useState, useCallback, ReactNode } from "react";
import type { Finding, Severity } from "../types";

interface ScanContextType {
  // State
  logs: string[];
  isAnalyzing: boolean;
  findings: Finding[];
  error: string | null;
  selectedFile: File | null;
  severityFilter: Severity | "all";
  
  hasRunScan: boolean;
  setHasRunScan: (hasRun: boolean) => void;
  
  // Actions
  addLog: (text: string) => void;
  setLogs: (logs: string[]) => void;
  setIsAnalyzing: (isAnalyzing: boolean) => void;
  setFindings: (findings: Finding[]) => void;
  setError: (error: string | null) => void;
  setSelectedFile: (file: File | null) => void;
  setSeverityFilter: (filter: Severity | "all") => void;
  resetScan: () => void;
}

const ScanContext = createContext<ScanContextType | undefined>(undefined);

export function ScanProvider({ children }: { children: ReactNode }) {
  const [logs, setLogs] = useState<string[]>([]);
  const [isAnalyzing, setIsAnalyzing] = useState(false);
  const [findings, setFindings] = useState<Finding[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [selectedFile, setSelectedFile] = useState<File | null>(null);
  const [severityFilter, setSeverityFilter] = useState<Severity | "all">("all");
  const [hasRunScan, setHasRunScan] = useState(false);

  const addLog = useCallback((text: string) => {
    setLogs((prev) => [...prev, `[${new Date().toLocaleTimeString()}] ${text}`]);
  }, []);

  const resetScan = useCallback(() => {
    setLogs([]);
    setFindings([]);
    setError(null);
    setIsAnalyzing(false);
    setHasRunScan(false);
  }, []);

  const value: ScanContextType = {
    logs,
    isAnalyzing,
    findings,
    error,
    selectedFile,
    hasRunScan,
    setHasRunScan,
    addLog,
    setLogs,
    setIsAnalyzing,
    setFindings,
    setError,
    setSelectedFile,
    setSeverityFilter,
    resetScan,
  };

  return <ScanContext.Provider value={value}>{children}</ScanContext.Provider>;
}

export function useScan() {
  const context = useContext(ScanContext);
  if (!context) {
    throw new Error("useScan must be used within ScanProvider");
  }
  return context;
}
