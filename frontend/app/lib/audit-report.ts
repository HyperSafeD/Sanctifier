import type { Finding, Severity, WorkspaceSummary } from "../types";
import { transformReport } from "./transform";
import { calculateScore } from "./export-pdf";

export const SEVERITIES: Severity[] = ["critical", "high", "medium", "low"];

const SEVERITY_RANK: Record<Severity, number> = { critical: 3, high: 2, medium: 1, low: 0 };

export type SeverityCounts = Record<Severity, number>;

export interface ContractAudit {
  name: string;
  total: number;
  counts: SeverityCounts;
  score: number;
}

export interface RemediationStep {
  severity: Severity;
  window: string;
  count: number;
}

export interface AuditReport {
  contracts: ContractAudit[];
  findings: Finding[];
  totals: SeverityCounts & { all: number };
  score: number;
  riskLabel: "Critical" | "High" | "Moderate" | "Low";
  topFindings: Finding[];
  timeline: RemediationStep[];
}

const REMEDIATION_WINDOWS: Record<Severity, string> = {
  critical: "Immediately (within 24 hours)",
  high: "Within 1 week",
  medium: "Within 30 days",
  low: "Backlog / next maintenance cycle",
};

export function emptyCounts(): SeverityCounts {
  return { critical: 0, high: 0, medium: 0, low: 0 };
}

export function countBySeverity(findings: Finding[]): SeverityCounts {
  const counts = emptyCounts();
  for (const f of findings) counts[f.severity] += 1;
  return counts;
}

export function riskLabelFor(score: number): AuditReport["riskLabel"] {
  if (score < 40) return "Critical";
  if (score < 60) return "High";
  if (score < 80) return "Moderate";
  return "Low";
}

/** Aggregate a workspace's per-contract reports into the audit report model. */
export function buildAuditReport(workspace: WorkspaceSummary): AuditReport {
  const findings: Finding[] = [];
  const contracts: ContractAudit[] = workspace.contracts.map((member) => {
    const memberFindings = member.report ? transformReport(member.report) : [];
    // Ids are only unique within one report, so namespace them by contract.
    findings.push(...memberFindings.map((f) => ({ ...f, id: `${member.name}:${f.id}` })));
    return {
      name: member.name,
      total: memberFindings.length,
      counts: countBySeverity(memberFindings),
      score: calculateScore(memberFindings),
    };
  });

  const counts = countBySeverity(findings);
  const score = calculateScore(findings);

  const topFindings = [...findings]
    .sort((a, b) => SEVERITY_RANK[b.severity] - SEVERITY_RANK[a.severity] || a.code.localeCompare(b.code))
    .slice(0, 10);

  const timeline = SEVERITIES.filter((severity) => counts[severity] > 0).map((severity) => ({
    severity,
    window: REMEDIATION_WINDOWS[severity],
    count: counts[severity],
  }));

  return {
    contracts: contracts.sort((a, b) => a.score - b.score || a.name.localeCompare(b.name)),
    findings,
    totals: { ...counts, all: findings.length },
    score,
    riskLabel: riskLabelFor(score),
    topFindings,
    timeline,
  };
}
