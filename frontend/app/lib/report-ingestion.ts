import type { WorkspaceSummary } from "../types";
import { normalizeReport, transformReport } from "./transform";

export const SAMPLE_JSON = `{
  "size_warnings": [],
  "unsafe_patterns": [],
  "auth_gaps": [],
  "panic_issues": [],
  "arithmetic_issues": []
}`;

export function extractErrorMessage(payload: unknown, fallback: string): string {
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

export function isWorkspaceSummary(payload: unknown): payload is WorkspaceSummary {
  return (
    typeof payload === "object" &&
    payload !== null &&
    "contracts" in payload &&
    Array.isArray((payload as { contracts?: unknown }).contracts)
  );
}

export function createWorkspaceFromSingleReport(rawReport: unknown): WorkspaceSummary {
  const report = normalizeReport(rawReport);
  const findingCount = transformReport(report).length;

  return {
    workspace: "Uploaded Report",
    contracts: [
      {
        name: "current-contract",
        total_findings: findingCount,
        report,
      },
    ],
    shared_libs: [],
    grand_total_findings: findingCount,
  };
}

export function parseJsonInput(input: string): unknown {
  // If input is empty or whitespace-only, return sample JSON
  if (!input || input.trim() === "") {
    return JSON.parse(SAMPLE_JSON);
  }
  
  // Try to parse the input, let errors bubble up for invalid JSON
  return JSON.parse(input);
}
