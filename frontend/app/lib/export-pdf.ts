import type { AnalysisReport, Finding } from "../types";
import {
  buildAuditExportModel,
  getSeverityColor,
  orderedSeverities,
} from "./report-export";

export async function exportToPdf(
  findings: Finding[],
  report: AnalysisReport | null,
  title = "Sanctifier Compliance Audit Report"
): Promise<void> {
  try {
    const { jsPDF } = await import("jspdf");
    const doc = new jsPDF();
    const model = buildAuditExportModel(findings, report, title);
    const accent = getSeverityColor(model.score);
    const pageWidth = doc.internal.pageSize.getWidth();
    const pageHeight = doc.internal.pageSize.getHeight();
    let pageNum = 1;

    const addFooter = () => {
      doc.setFontSize(8);
      doc.setFont("helvetica", "normal");
      doc.setTextColor(150);
      doc.text(`${model.title} - Page ${pageNum}`, pageWidth / 2, pageHeight - 8, {
        align: "center",
      });
      doc.setTextColor(24, 24, 27);
    };

    doc.setFillColor(15, 23, 42);
    doc.rect(0, 0, pageWidth, 42, "F");

    doc.setTextColor(255, 255, 255);
    doc.setFontSize(20);
    doc.setFont("helvetica", "bold");
    doc.text(model.title, 14, 18);

    doc.setFontSize(10);
    doc.setFont("helvetica", "normal");
    doc.text(`Generated: ${model.generatedAt}`, 14, 26);
    doc.text("Prepared for compliance and audit review", 14, 32);

    doc.setFillColor(accent);
    doc.roundedRect(pageWidth - 58, 10, 42, 20, 4, 4, "F");
    doc.setFontSize(18);
    doc.setFont("helvetica", "bold");
    doc.text(String(model.score), pageWidth - 37, 20, { align: "center" });
    doc.setFontSize(8);
    doc.text(`Grade ${model.grade}`, pageWidth - 37, 26, { align: "center" });

    doc.setTextColor(24, 24, 27);
    let y = 54;

    doc.setFontSize(12);
    doc.setFont("helvetica", "bold");
    doc.text("Executive Summary", 14, y);
    y += 8;

    doc.setFontSize(10);
    doc.setFont("helvetica", "normal");
    const summary = doc.splitTextToSize(
      `${model.narrative}. Sanctifier identified ${model.totalFindings} total findings in this report. ` +
        "Use the severity summary and detailed findings below to support remediation planning and audit evidence collection.",
      182
    );
    doc.text(summary, 14, y);
    y += summary.length * 5 + 6;

    doc.setFontSize(12);
    doc.setFont("helvetica", "bold");
    doc.text("Severity Summary", 14, y);
    y += 8;

    doc.setFontSize(10);
    doc.setFont("helvetica", "normal");
    orderedSeverities().forEach((severity) => {
      const label = severity.charAt(0).toUpperCase() + severity.slice(1);
      doc.text(`${label}: ${model.severityCounts[severity]}`, 14, y);
      y += 5;
    });
    y += 6;

    doc.setDrawColor(200);
    doc.line(14, y, pageWidth - 14, y);
    y += 10;

    doc.setFontSize(12);
    doc.setFont("helvetica", "bold");
    doc.text("Methodology", 14, y);
    y += 8;

    doc.setFontSize(10);
    doc.setFont("helvetica", "normal");
    model.methodology.forEach((step) => {
      const lines = doc.splitTextToSize(`- ${step}`, 182);
      doc.text(lines, 14, y);
      y += lines.length * 5 + 2;
    });
    y += 4;

    addFooter();

    orderedSeverities().forEach((severity) => {
      const groupedFindings = model.findingsBySeverity[severity];
      if (groupedFindings.length === 0) {
        return;
      }

      if (y > 250) {
        doc.addPage();
        pageNum += 1;
        y = 20;
        addFooter();
      }

      const label = severity.charAt(0).toUpperCase() + severity.slice(1);
      doc.setFontSize(13);
      doc.setFont("helvetica", "bold");
      doc.text(`${label} Findings (${groupedFindings.length})`, 14, y);
      y += 8;

      groupedFindings.forEach((finding, index) => {
        if (y > 260) {
          doc.addPage();
          pageNum += 1;
          y = 20;
          addFooter();
        }

        doc.setFontSize(11);
        doc.setFont("helvetica", "bold");
        doc.text(`${index + 1}. ${finding.title}`, 14, y);
        y += 6;

        doc.setFont("helvetica", "normal");
        doc.setFontSize(9);
        doc.text(`Category: ${finding.category}`, 20, y);
        y += 5;
        doc.text(`Code: ${finding.code}`, 20, y);
        y += 5;
        doc.text(`Location: ${finding.location}`, 20, y);
        y += 5;

        if (finding.snippet) {
          const snippetLines = doc.splitTextToSize(`Code: ${finding.snippet}`, 170);
          doc.text(snippetLines, 20, y);
          y += snippetLines.length * 4;
        }

        if (finding.suggestion) {
          const suggestionLines = doc.splitTextToSize(
            `Suggestion: ${finding.suggestion}`,
            170
          );
          doc.text(suggestionLines, 20, y);
          y += suggestionLines.length * 4;
        }

        y += 6;
      });

      y += 4;
    });

    doc.save("sanctifier-report.pdf");
  } catch {
    window.print();
  }
}
