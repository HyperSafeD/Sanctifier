import type { AnalysisReport, Finding, Severity } from "../types";

const SEVERITY_WEIGHTS: Record<Severity, number> = {
  critical: 15,
  high: 10,
  medium: 5,
  low: 2,
};

const SEVERITY_ORDER: Severity[] = ["critical", "high", "medium", "low"];

export interface AuditExportModel {
  title: string;
  generatedAt: string;
  score: number;
  grade: string;
  narrative: string;
  severityCounts: Record<Severity, number>;
  totalFindings: number;
  methodology: string[];
  findingsBySeverity: Record<Severity, Finding[]>;
  findings: Finding[];
  report: AnalysisReport | null;
}

export function calculateScore(findings: Finding[]): number {
  let score = 100;
  for (const finding of findings) {
    score -= SEVERITY_WEIGHTS[finding.severity] ?? 0;
  }
  return Math.max(0, Math.min(100, score));
}

export function getScoreGrade(score: number): string {
  if (score >= 90) return "A";
  if (score >= 80) return "B";
  if (score >= 65) return "C";
  if (score >= 50) return "D";
  return "F";
}

export function getScoreNarrative(score: number): string {
  if (score >= 76) return "Good security posture";
  if (score >= 50) return "Moderate risk - review findings";
  return "High risk - immediate attention needed";
}

export function getSeverityColor(score: number): string {
  if (score >= 76) return "#22c55e";
  if (score >= 61) return "#f59e0b";
  if (score >= 41) return "#f97316";
  return "#ef4444";
}

export function buildAuditExportModel(
  findings: Finding[],
  report: AnalysisReport | null,
  title = "Sanctifier Compliance Audit Report"
): AuditExportModel {
  const score = calculateScore(findings);
  const severityCounts = createSeverityCounts(findings);
  const findingsBySeverity = createSeverityBuckets(findings);

  return {
    title,
    generatedAt: new Date().toLocaleString(),
    score,
    grade: getScoreGrade(score),
    narrative: getScoreNarrative(score),
    severityCounts,
    totalFindings: findings.length,
    methodology: [
      "Static analysis findings are normalized from Sanctifier JSON output before export.",
      "Sanctity Score starts at 100 and is reduced by weighted severity deductions.",
      "Critical and high findings should be reviewed before production deployment.",
      "This document is intended for audit, compliance, and remediation tracking workflows.",
    ],
    findingsBySeverity,
    findings,
    report,
  };
}

export function buildCsvContent(
  findings: Finding[],
  report: AnalysisReport | null,
  title = "Sanctifier Compliance Audit Report"
): string {
  const model = buildAuditExportModel(findings, report, title);
  const rows = [
    [
      "report_title",
      "generated_at",
      "sanctity_score",
      "grade",
      "severity",
      "code",
      "category",
      "title",
      "location",
      "suggestion",
      "snippet",
    ],
    ...model.findings.map((finding) => [
      model.title,
      model.generatedAt,
      String(model.score),
      model.grade,
      finding.severity,
      finding.code,
      finding.category,
      finding.title,
      finding.location,
      finding.suggestion ?? "",
      finding.snippet ?? "",
    ]),
  ];

  return rows.map((row) => row.map(escapeCsvCell).join(",")).join("\n");
}

export function orderedSeverities(): Severity[] {
  return [...SEVERITY_ORDER];
}

function createSeverityCounts(findings: Finding[]): Record<Severity, number> {
  const counts: Record<Severity, number> = {
    critical: 0,
    high: 0,
    medium: 0,
    low: 0,
  };

  findings.forEach((finding) => {
    counts[finding.severity] += 1;
  });

  return counts;
}

function createSeverityBuckets(findings: Finding[]): Record<Severity, Finding[]> {
  return {
    critical: findings.filter((finding) => finding.severity === "critical"),
    high: findings.filter((finding) => finding.severity === "high"),
    medium: findings.filter((finding) => finding.severity === "medium"),
    low: findings.filter((finding) => finding.severity === "low"),
  };
}

function escapeCsvCell(value: string): string {
  const normalized = value.replace(/\r?\n/g, " ").replace(/"/g, '""');
  return `"${normalized}"`;
}
