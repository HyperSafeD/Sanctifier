import type { AnalysisReport, Finding, ReportDiff, DiffStatus, Severity } from "../types";
import { normalizeReport, transformReport } from "./transform";
import { canonicalizeFindingCode } from "./finding-filters";

interface FindingFingerprint {
  code: string;
  location: string;
  category: string;
  title: string;
}

function fingerprint(finding: Finding): string {
  const code = canonicalizeFindingCode(finding.code);
  return `${code}::${finding.location}::${finding.category}`;
}

function extractKey(finding: Finding): FindingFingerprint {
  return {
    code: canonicalizeFindingCode(finding.code),
    location: finding.location,
    category: finding.category,
    title: finding.title,
  };
}

function fingerprintsMatch(a: Finding, b: Finding): boolean {
  return fingerprint(a) === fingerprint(b);
}

export function computeReportDiff(
  baselineReport: AnalysisReport | null,
  currentReport: AnalysisReport | null,
  baselineName = "Baseline",
  currentName = "Current"
): ReportDiff | null {
  if (!baselineReport && !currentReport) return null;

  const baselineFindings = baselineReport ? transformReport(normalizeReport(baselineReport)) : [];
  const currentFindings = currentReport ? transformReport(normalizeReport(currentReport)) : [];

  const baselineMap = new Map<string, Finding>();
  for (const f of baselineFindings) {
    baselineMap.set(fingerprint(f), f);
  }

  const currentMap = new Map<string, Finding>();
  for (const f of currentFindings) {
    currentMap.set(fingerprint(f), f);
  }

  const processed = new Set<string>();
  const diffFindings: Array<{ finding: Finding; status: DiffStatus; previousSeverity?: Severity }> = [];

  for (const f of currentFindings) {
    const fp = fingerprint(f);
    processed.add(fp);
    if (baselineMap.has(fp)) {
      const prev = baselineMap.get(fp)!;
      if (prev.severity !== f.severity) {
        diffFindings.push({ finding: f, status: "severity_changed", previousSeverity: prev.severity });
      } else {
        diffFindings.push({ finding: f, status: "unchanged" });
      }
    } else {
      diffFindings.push({ finding: f, status: "added" });
    }
  }

  for (const f of baselineFindings) {
    const fp = fingerprint(f);
    if (!processed.has(fp)) {
      diffFindings.push({ finding: f, status: "removed" });
    }
  }

  const addedCount = diffFindings.filter((d) => d.status === "added").length;
  const removedCount = diffFindings.filter((d) => d.status === "removed").length;
  const unchangedCount = diffFindings.filter((d) => d.status === "unchanged").length;
  const severityChangedCount = diffFindings.filter((d) => d.status === "severity_changed").length;

  return {
    baselineName,
    currentName,
    baselineFindings,
    currentFindings,
    diffFindings,
    addedCount,
    removedCount,
    unchangedCount,
    severityChangedCount,
  };
}

export interface MergedDiffJson {
  schema_version: string;
  baseline_name: string;
  current_name: string;
  summary: {
    baseline_findings: number;
    current_findings: number;
    added: number;
    removed: number;
    unchanged: number;
    severity_changed: number;
  };
  diff: Array<{
    status: string;
    previous_severity: string | null;
    code: string;
    severity: string;
    category: string;
    title: string;
    location: string;
    snippet: string | null;
    suggestion: string | null;
  }>;
}

export function exportDiffAsJson(diff: ReportDiff): MergedDiffJson {
  return {
    schema_version: "1.0.0",
    baseline_name: diff.baselineName,
    current_name: diff.currentName,
    summary: {
      baseline_findings: diff.baselineFindings.length,
      current_findings: diff.currentFindings.length,
      added: diff.addedCount,
      removed: diff.removedCount,
      unchanged: diff.unchangedCount,
      severity_changed: diff.severityChangedCount,
    },
    diff: diff.diffFindings.map((d) => ({
      status: d.status,
      previous_severity: d.previousSeverity ?? null,
      code: d.finding.code,
      severity: d.finding.severity,
      category: d.finding.category,
      title: d.finding.title,
      location: d.finding.location,
      snippet: d.finding.snippet ?? null,
      suggestion: d.finding.suggestion ?? null,
    })),
  };
}
