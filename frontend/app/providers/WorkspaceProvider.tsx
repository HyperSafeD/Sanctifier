"use client";

import React, {
  createContext, useContext, useState,
  useCallback, useMemo, useEffect,
} from "react";
import type { WorkspaceSummary, WorkspaceMember, AnalysisReport } from "../types";

const STORAGE_KEY_WORKSPACE = "sanctifier_workspace";
const STORAGE_KEY_CONTRACT  = "sanctifier_selected_contract";
const SIZE_LIMIT_BYTES = 2 * 1024 * 1024; // 2 MB — fall back to sessionStorage above this

interface WorkspaceContextType {
  workspace: WorkspaceSummary | null;
  selectedContract: WorkspaceMember | null;
  setWorkspace: (w: WorkspaceSummary | null) => void;
  selectContract: (name: string) => void;
  updateContractReport: (name: string, report: AnalysisReport) => void;
  clearWorkspace: () => void;
}

const WorkspaceContext = createContext<WorkspaceContextType | undefined>(undefined);

// ── Storage helpers (SSR-safe) ────────────────────────────────────────────────

function safeWrite(key: string, value: string): void {
  if (typeof window === "undefined") return;
  try {
    const bytes = new TextEncoder().encode(value).length;
    const storage = bytes > SIZE_LIMIT_BYTES ? window.sessionStorage : window.localStorage;
    storage.setItem(key, value);
    // Remove from the other store to avoid stale data
    const other = bytes > SIZE_LIMIT_BYTES ? window.localStorage : window.sessionStorage;
    other.removeItem(key);
  } catch {
    // Storage quota exceeded — silently ignore
  }
}

function safeRead(key: string): string | null {
  if (typeof window === "undefined") return null;
  return window.localStorage.getItem(key) ?? window.sessionStorage.getItem(key);
}

function safeRemove(key: string): void {
  if (typeof window === "undefined") return;
  window.localStorage.removeItem(key);
  window.sessionStorage.removeItem(key);
}

// ── Provider ──────────────────────────────────────────────────────────────────

export function WorkspaceProvider({ children }: { children: React.ReactNode }) {
  const [workspace, setWorkspaceState] = useState<WorkspaceSummary | null>(null);
  const [selectedContractName, setSelectedContractName] = useState<string | null>(null);
  const [hydrated, setHydrated] = useState(false);

  // Hydrate from storage on mount (client-only)
  useEffect(() => {
    const raw = safeRead(STORAGE_KEY_WORKSPACE);
    if (raw) {
      try {
        const parsed: WorkspaceSummary = JSON.parse(raw);
        setWorkspaceState(parsed);
        const saved = safeRead(STORAGE_KEY_CONTRACT);
        const first = parsed.contracts[0]?.name ?? null;
        setSelectedContractName(saved ?? first);
      } catch {
        safeRemove(STORAGE_KEY_WORKSPACE);
        safeRemove(STORAGE_KEY_CONTRACT);
      }
    }
    setHydrated(true);
  }, []);

  // Persist workspace whenever it changes (after hydration)
  useEffect(() => {
    if (!hydrated) return;
    if (workspace) {
      safeWrite(STORAGE_KEY_WORKSPACE, JSON.stringify(workspace));
    } else {
      safeRemove(STORAGE_KEY_WORKSPACE);
    }
  }, [workspace, hydrated]);

  // Persist selected contract name
  useEffect(() => {
    if (!hydrated) return;
    if (selectedContractName) {
      safeWrite(STORAGE_KEY_CONTRACT, selectedContractName);
    } else {
      safeRemove(STORAGE_KEY_CONTRACT);
    }
  }, [selectedContractName, hydrated]);

  const setWorkspace = useCallback((w: WorkspaceSummary | null) => {
    setWorkspaceState(w);
    setSelectedContractName(w?.contracts[0]?.name ?? null);
  }, []);

  const selectContract = useCallback((name: string) => {
    setSelectedContractName(name);
  }, []);

  const updateContractReport = useCallback((name: string, report: AnalysisReport) => {
    setWorkspaceState((prev) => {
      if (!prev) return null;
      return { ...prev, contracts: prev.contracts.map((c) => c.name === name ? { ...c, report } : c) };
    });
  }, []);

  const clearWorkspace = useCallback(() => {
    setWorkspaceState(null);
    setSelectedContractName(null);
    safeRemove(STORAGE_KEY_WORKSPACE);
    safeRemove(STORAGE_KEY_CONTRACT);
  }, []);

  const selectedContract = useMemo(() => {
    if (!workspace || !selectedContractName) return null;
    return workspace.contracts.find((c) => c.name === selectedContractName) ?? null;
  }, [workspace, selectedContractName]);

  const value = useMemo(
    () => ({ workspace, selectedContract, setWorkspace, selectContract, updateContractReport, clearWorkspace }),
    [workspace, selectedContract, setWorkspace, selectContract, updateContractReport, clearWorkspace]
  );

  return <WorkspaceContext.Provider value={value}>{children}</WorkspaceContext.Provider>;
}

export function useWorkspace() {
  const ctx = useContext(WorkspaceContext);
  if (!ctx) throw new Error("useWorkspace must be used within WorkspaceProvider");
  return ctx;
}
