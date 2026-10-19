import type { WorkspaceSummary, AnalysisReport } from "../types";

const PARAM_KEY = "report";
const MAX_URL_BYTES = 8000;

function isWorkspaceSummary(v: unknown): v is WorkspaceSummary {
  return (
    typeof v === "object" &&
    v !== null &&
    "workspace" in v &&
    "contracts" in v
  );
}

export function encodeShareLink(data: WorkspaceSummary | AnalysisReport): string {
  const json = JSON.stringify(data);
  const compressed = btoa(encodeURIComponent(json));
  const url = new URL(window.location.href);
  url.searchParams.set(PARAM_KEY, compressed);
  return url.toString();
}

export function decodeShareLink(search: string): WorkspaceSummary | AnalysisReport | null {
  try {
    const params = new URLSearchParams(search);
    const encoded = params.get(PARAM_KEY);
    if (!encoded) return null;
    const json = decodeURIComponent(atob(encoded));
    const parsed: unknown = JSON.parse(json);
    if (typeof parsed !== "object" || parsed === null) return null;
    return parsed as WorkspaceSummary | AnalysisReport;
  } catch {
    return null;
  }
}

export function isShareLinkTooLarge(data: WorkspaceSummary | AnalysisReport): boolean {
  const json = JSON.stringify(data);
  const compressed = btoa(encodeURIComponent(json));
  return compressed.length > MAX_URL_BYTES;
}

export async function copyShareLink(data: WorkspaceSummary | AnalysisReport): Promise<void> {
  const link = encodeShareLink(data);
  await navigator.clipboard.writeText(link);
}
