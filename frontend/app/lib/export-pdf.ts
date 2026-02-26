import type { Finding, Severity } from "../types";

const SEVERITY_WEIGHTS: Record<string, number> = {
  critical: 15,
  high: 10,
  medium: 5,
  low: 2,
};

function calculateScore(findings: Finding[]): number {
  let score = 100;
  for (const f of findings) {
    score -= SEVERITY_WEIGHTS[f.severity] ?? 0;
  }
  return Math.max(0, Math.min(100, score));
}

export async function exportToPdf(
  findings: Finding[],
  title = "Sanctifier Security Report"
): Promise<void> {
  try {
    const { jsPDF } = await import("jspdf");
    const doc = new jsPDF();
    let pageNum = 1;

    const addFooter = () => {
      doc.setFontSize(8);
      doc.setFont("helvetica", "normal");
      doc.setTextColor(150);
      doc.text(
        `Sanctifier Security Report - Page ${pageNum}`,
        105,
        290,
        { align: "center" }
      );
      doc.setTextColor(0);
    };

    // Header
    doc.setFontSize(20);
    doc.setFont("helvetica", "bold");
    doc.text(title, 14, 22);

    doc.setFontSize(10);
    doc.setFont("helvetica", "normal");
    doc.text(`Generated: ${new Date().toLocaleString()}`, 14, 30);
    doc.text(`Total findings: ${findings.length}`, 14, 36);

    // Sanctity Score
    const score = calculateScore(findings);
    doc.setFontSize(14);
    doc.setFont("helvetica", "bold");
    doc.text(`Sanctity Score: ${score}/100`, 14, 48);

    // Severity summary table
    const severities: Severity[] = ["critical", "high", "medium", "low"];
    const counts: Record<string, number> = { critical: 0, high: 0, medium: 0, low: 0 };
    findings.forEach((f) => { counts[f.severity]++; });

    let y = 58;
    doc.setFontSize(12);
    doc.setFont("helvetica", "bold");
    doc.text("Summary", 14, y);
    y += 8;

    doc.setFontSize(10);
    doc.setFont("helvetica", "normal");
    severities.forEach((sev) => {
      doc.text(`${sev.charAt(0).toUpperCase() + sev.slice(1)}: ${counts[sev]}`, 14, y);
      y += 6;
    });
    y += 6;

    // Separator line
    doc.setDrawColor(200);
    doc.line(14, y, 196, y);
    y += 10;

    // Findings grouped by severity
    addFooter();

    severities.forEach((sev) => {
      const sevFindings = findings.filter((f) => f.severity === sev);
      if (sevFindings.length === 0) return;

      if (y > 250) {
        doc.addPage();
        pageNum++;
        y = 20;
        addFooter();
      }

      // Section header
      doc.setFontSize(13);
      doc.setFont("helvetica", "bold");
      doc.text(
        `${sev.charAt(0).toUpperCase() + sev.slice(1)} (${sevFindings.length})`,
        14,
        y
      );
      y += 8;

      sevFindings.forEach((f, i) => {
        if (y > 260) {
          doc.addPage();
          pageNum++;
          y = 20;
          addFooter();
        }

        doc.setFontSize(11);
        doc.setFont("helvetica", "bold");
        doc.text(`${i + 1}. ${f.title}`, 14, y);
        y += 6;

        doc.setFont("helvetica", "normal");
        doc.setFontSize(9);
        doc.text(`Category: ${f.category}`, 20, y);
        y += 5;
        doc.text(`Location: ${f.location}`, 20, y);
        y += 5;

        if (f.snippet) {
          const snippetLines = doc.splitTextToSize(`Code: ${f.snippet}`, 170);
          doc.text(snippetLines, 20, y);
          y += snippetLines.length * 4;
        }
        if (f.suggestion) {
          const suggLines = doc.splitTextToSize(`Suggestion: ${f.suggestion}`, 170);
          doc.text(suggLines, 20, y);
          y += suggLines.length * 4;
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
