import type { AnalysisReport, Finding } from "../types";
import { buildCsvContent } from "./report-export";

export function exportToCsv(
  findings: Finding[],
  report: AnalysisReport | null,
  title = "Sanctifier Compliance Audit Report"
): void {
  const csv = buildCsvContent(findings, report, title);
  const blob = new Blob([csv], { type: "text/csv;charset=utf-8;" });
  const url = URL.createObjectURL(blob);
  const link = document.createElement("a");

  link.href = url;
  link.download = "sanctifier-report.csv";
  link.click();

  URL.revokeObjectURL(url);
}
