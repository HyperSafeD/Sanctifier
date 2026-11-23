"use client";

import React, {
  createContext, useContext, useReducer, useMemo, useCallback,
} from "react";
import type { Severity } from "../types";
import type { FileProgress } from "../components/DashboardHeader";
import type { RejectedFile } from "../lib/upload-validation";
import type { ScanRecord } from "../lib/scan-history";

/** The three analysis views the dashboard can show. */
export type DashboardTab = "findings" | "callgraph" | "diff";

/**
 * All view state owned by the dashboard.
 *
 * Report data itself lives in `WorkspaceProvider` — this store only holds the
 * state that describes how the report is being *looked at*: filters, the
 * active tab, in-flight upload progress, and client-side caches.
 */
export interface DashboardState {
  /** Severity currently selected in the filter bar. */
  severityFilter: Severity | "all";
  /** Which analysis tab is visible. */
  activeTab: DashboardTab;
  /** Fatal message shown above the report, or `null` when healthy. */
  error: string | null;
  /** Raw JSON in the report textarea. */
  jsonInput: string;
  /** Raw JSON in the diff view's baseline textarea. */
  baselineJsonInput: string;
  /** Transient progress message for the current upload, or `null`. */
  uploadStatus: string | null;
  /** True while contract files are being analysed. */
  isUploadingContract: boolean;
  /** Normalised finding-code query (e.g. `"S001"`). */
  codeFilterInput: string;
  /** Validation message for `codeFilterInput`, or `null` when it is valid. */
  codeFilterError: string | null;
  /** Whether the mobile workspace sidebar is open. */
  sidebarOpen: boolean;
  /** Per-file analysis progress, keyed by file name. */
  batchProgress: Record<string, FileProgress>;
  /** Files rejected by upload validation, shown briefly then cleared. */
  rejectedFiles: RejectedFile[];
  /** Persisted scan history powering the trend chart. */
  trendRecords: ScanRecord[];
  /** Uploaded contract sources, keyed by file name. */
  sourceCache: Record<string, string>;
}

/**
 * Dashboard state transitions.
 *
 * Multi-field transitions are modelled as one domain event rather than a
 * cascade of setters, so a screen can never observe a half-applied upload —
 * `uploadFailed`, for instance, clears the status and lowers the in-flight
 * flag in the same commit.
 */
export type DashboardAction =
  | { type: "setSeverityFilter"; severityFilter: Severity | "all" }
  | { type: "setActiveTab"; activeTab: DashboardTab }
  | { type: "setJsonInput"; jsonInput: string }
  | { type: "setBaselineJsonInput"; baselineJsonInput: string }
  | { type: "setCodeFilter"; codeFilterInput: string; codeFilterError: string | null }
  | { type: "setSidebarOpen"; sidebarOpen: boolean }
  | { type: "setError"; error: string | null }
  | { type: "setTrendRecords"; trendRecords: ScanRecord[] }
  | { type: "cacheSources"; sources: Record<string, string> }
  | { type: "rejectFiles"; rejectedFiles: RejectedFile[] }
  | { type: "clearRejectedFiles" }
  | { type: "parseStarted" }
  | { type: "parseFailed"; error: string }
  | { type: "uploadStarted"; batchProgress: Record<string, FileProgress>; uploadStatus: string }
  | { type: "fileProgress"; fileName: string; progress: FileProgress }
  | { type: "uploadSucceeded"; uploadStatus: string }
  | { type: "uploadFailed"; error: string };

export const initialDashboardState: DashboardState = {
  severityFilter: "all",
  activeTab: "findings",
  error: null,
  jsonInput: "",
  baselineJsonInput: "",
  uploadStatus: null,
  isUploadingContract: false,
  codeFilterInput: "",
  codeFilterError: null,
  sidebarOpen: false,
  batchProgress: {},
  rejectedFiles: [],
  trendRecords: [],
  sourceCache: {},
};

/**
 * Pure reducer for {@link DashboardState}.
 *
 * Exported so the transitions can be unit-tested without mounting a tree.
 */
export function dashboardReducer(
  state: DashboardState,
  action: DashboardAction,
): DashboardState {
  switch (action.type) {
    case "setSeverityFilter":
      return { ...state, severityFilter: action.severityFilter };
    case "setActiveTab":
      return { ...state, activeTab: action.activeTab };
    case "setJsonInput":
      return { ...state, jsonInput: action.jsonInput };
    case "setBaselineJsonInput":
      return { ...state, baselineJsonInput: action.baselineJsonInput };
    case "setCodeFilter":
      return {
        ...state,
        codeFilterInput: action.codeFilterInput,
        codeFilterError: action.codeFilterError,
      };
    case "setSidebarOpen":
      return { ...state, sidebarOpen: action.sidebarOpen };
    case "setError":
      return { ...state, error: action.error };
    case "setTrendRecords":
      return { ...state, trendRecords: action.trendRecords };
    case "cacheSources":
      return { ...state, sourceCache: { ...state.sourceCache, ...action.sources } };
    case "rejectFiles":
      return { ...state, rejectedFiles: action.rejectedFiles };
    case "clearRejectedFiles":
      return { ...state, rejectedFiles: [] };
    case "parseStarted":
      return { ...state, error: null, uploadStatus: null };
    case "parseFailed":
      return { ...state, error: action.error, uploadStatus: null };
    case "uploadStarted":
      return {
        ...state,
        error: null,
        isUploadingContract: true,
        batchProgress: action.batchProgress,
        uploadStatus: action.uploadStatus,
      };
    case "fileProgress":
      return {
        ...state,
        batchProgress: { ...state.batchProgress, [action.fileName]: action.progress },
      };
    case "uploadSucceeded":
      return {
        ...state,
        isUploadingContract: false,
        error: null,
        uploadStatus: action.uploadStatus,
      };
    case "uploadFailed":
      return {
        ...state,
        isUploadingContract: false,
        uploadStatus: null,
        error: action.error,
      };
    default:
      return state;
  }
}

/** Memoised action creators bound to the provider's `dispatch`. */
export interface DashboardActions {
  setSeverityFilter: (severityFilter: Severity | "all") => void;
  setActiveTab: (activeTab: DashboardTab) => void;
  setJsonInput: (jsonInput: string) => void;
  setBaselineJsonInput: (baselineJsonInput: string) => void;
  setCodeFilter: (codeFilterInput: string, codeFilterError: string | null) => void;
  setSidebarOpen: (sidebarOpen: boolean) => void;
  setError: (error: string | null) => void;
  setTrendRecords: (trendRecords: ScanRecord[]) => void;
  cacheSources: (sources: Record<string, string>) => void;
  rejectFiles: (rejectedFiles: RejectedFile[]) => void;
  clearRejectedFiles: () => void;
  parseStarted: () => void;
  parseFailed: (error: string) => void;
  uploadStarted: (batchProgress: Record<string, FileProgress>, uploadStatus: string) => void;
  fileProgress: (fileName: string, progress: FileProgress) => void;
  uploadSucceeded: (uploadStatus: string) => void;
  uploadFailed: (error: string) => void;
}

// State and actions live in separate contexts on purpose: `actions` is stable
// for the lifetime of the provider, so a consumer that only dispatches (a
// toolbar button, say) never re-renders when a keystroke changes `jsonInput`.
const DashboardStateContext = createContext<DashboardState | undefined>(undefined);
const DashboardActionsContext = createContext<DashboardActions | undefined>(undefined);

export function DashboardProvider({
  children,
  initialState = initialDashboardState,
}: {
  children: React.ReactNode;
  /** Seed state — used by tests and by Storybook to render a given view. */
  initialState?: DashboardState;
}) {
  const [state, dispatch] = useReducer(dashboardReducer, initialState);

  const setSeverityFilter = useCallback(
    (severityFilter: Severity | "all") => dispatch({ type: "setSeverityFilter", severityFilter }),
    [],
  );
  const setActiveTab = useCallback(
    (activeTab: DashboardTab) => dispatch({ type: "setActiveTab", activeTab }),
    [],
  );
  const setJsonInput = useCallback(
    (jsonInput: string) => dispatch({ type: "setJsonInput", jsonInput }),
    [],
  );
  const setBaselineJsonInput = useCallback(
    (baselineJsonInput: string) => dispatch({ type: "setBaselineJsonInput", baselineJsonInput }),
    [],
  );
  const setCodeFilter = useCallback(
    (codeFilterInput: string, codeFilterError: string | null) =>
      dispatch({ type: "setCodeFilter", codeFilterInput, codeFilterError }),
    [],
  );
  const setSidebarOpen = useCallback(
    (sidebarOpen: boolean) => dispatch({ type: "setSidebarOpen", sidebarOpen }),
    [],
  );
  const setError = useCallback(
    (error: string | null) => dispatch({ type: "setError", error }),
    [],
  );
  const setTrendRecords = useCallback(
    (trendRecords: ScanRecord[]) => dispatch({ type: "setTrendRecords", trendRecords }),
    [],
  );
  const cacheSources = useCallback(
    (sources: Record<string, string>) => dispatch({ type: "cacheSources", sources }),
    [],
  );
  const rejectFiles = useCallback(
    (rejectedFiles: RejectedFile[]) => dispatch({ type: "rejectFiles", rejectedFiles }),
    [],
  );
  const clearRejectedFiles = useCallback(() => dispatch({ type: "clearRejectedFiles" }), []);
  const parseStarted = useCallback(() => dispatch({ type: "parseStarted" }), []);
  const parseFailed = useCallback(
    (error: string) => dispatch({ type: "parseFailed", error }),
    [],
  );
  const uploadStarted = useCallback(
    (batchProgress: Record<string, FileProgress>, uploadStatus: string) =>
      dispatch({ type: "uploadStarted", batchProgress, uploadStatus }),
    [],
  );
  const fileProgress = useCallback(
    (fileName: string, progress: FileProgress) =>
      dispatch({ type: "fileProgress", fileName, progress }),
    [],
  );
  const uploadSucceeded = useCallback(
    (uploadStatus: string) => dispatch({ type: "uploadSucceeded", uploadStatus }),
    [],
  );
  const uploadFailed = useCallback(
    (error: string) => dispatch({ type: "uploadFailed", error }),
    [],
  );

  const actions = useMemo<DashboardActions>(
    () => ({
      setSeverityFilter,
      setActiveTab,
      setJsonInput,
      setBaselineJsonInput,
      setCodeFilter,
      setSidebarOpen,
      setError,
      setTrendRecords,
      cacheSources,
      rejectFiles,
      clearRejectedFiles,
      parseStarted,
      parseFailed,
      uploadStarted,
      fileProgress,
      uploadSucceeded,
      uploadFailed,
    }),
    [
      setSeverityFilter, setActiveTab, setJsonInput, setBaselineJsonInput,
      setCodeFilter, setSidebarOpen, setError, setTrendRecords, cacheSources,
      rejectFiles, clearRejectedFiles, parseStarted, parseFailed,
      uploadStarted, fileProgress, uploadSucceeded, uploadFailed,
    ],
  );

  return (
    <DashboardStateContext.Provider value={state}>
      <DashboardActionsContext.Provider value={actions}>
        {children}
      </DashboardActionsContext.Provider>
    </DashboardStateContext.Provider>
  );
}

/** Read the dashboard view state. Re-renders on every state change. */
export function useDashboardState(): DashboardState {
  const ctx = useContext(DashboardStateContext);
  if (!ctx) throw new Error("useDashboardState must be used within DashboardProvider");
  return ctx;
}

/** Read the dashboard action creators. Stable — never triggers a re-render. */
export function useDashboardActions(): DashboardActions {
  const ctx = useContext(DashboardActionsContext);
  if (!ctx) throw new Error("useDashboardActions must be used within DashboardProvider");
  return ctx;
}
