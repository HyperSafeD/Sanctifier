import type { Finding, Severity } from "../types";

const SEVERITY_WEIGHTS: Record<string, number> = {
  critical: 15,
  high: 10,
  medium: 5,
  low: 2,
};

const SEVERITY_COLORS: Record<string, [number, number, number]> = {
  critical: [220, 38, 38],
  high: [234, 88, 12],
  medium: [245, 158, 11],
  low: [34, 197, 94],
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
    const PAGE_HEIGHT = 297;
    const MARGIN_TOP = 20;
    const MARGIN_BOTTOM = 15;
    const CONTENT_HEIGHT = PAGE_HEIGHT - MARGIN_TOP - MARGIN_BOTTOM;

    const addHeader = () => {
      doc.setFontSize(16);
      doc.setFont("helvetica", "bold");
      doc.text("Sanctifier", 14, 12);
      doc.setFontSize(10);
      doc.setFont("helvetica", "normal");
      doc.text("Security Analysis Report", 14, 18);
      
      // Horizontal line
      doc.setDrawColor(200);
      doc.line(14, 20, 196, 20);
    };

    const addFooter = () => {
      doc.setFontSize(8);
      doc.setFont("helvetica", "normal");
      doc.setTextColor(150);
      const timestamp = new Date().toISOString();
      doc.text(timestamp, 14, PAGE_HEIGHT - 8);
      doc.text(`Page ${pageNum}`, 196, PAGE_HEIGHT - 8, { align: "right" });
      doc.setTextColor(0);
    };

    // First page header
    addHeader();
    let y = 28;

    // Title and metadata
    doc.setFontSize(18);
    doc.setFont("helvetica", "bold");
    doc.text(title, 14, y);
    y += 10;

    doc.setFontSize(10);
    doc.setFont("helvetica", "normal");
    doc.text(`Generated: ${new Date().toLocaleString()}`, 14, y);
    y += 6;
    doc.text(`Total findings: ${findings.length}`, 14, y);
    y += 10;

    // Sanctity Score
    const score = calculateScore(findings);
    doc.setFontSize(14);
    doc.setFont("helvetica", "bold");
    doc.text(`Sanctity Score: ${score}/100`, 14, y);
    y += 10;

    // Severity summary table with colored badges
    const severities: Severity[] = ["critical", "high", "medium", "low"];
    const counts: Record<string, number> = { critical: 0, high: 0, medium: 0, low: 0 };
    findings.forEach((f) => { counts[f.severity]++; });

    doc.setFontSize(12);
    doc.setFont("helvetica", "bold");
    doc.text("Summary", 14, y);
    y += 8;

    doc.setFontSize(10);
    doc.setFont("helvetica", "normal");
    severities.forEach((sev) => {
      const [r, g, b] = SEVERITY_COLORS[sev];
      doc.setFillColor(r, g, b);
      doc.rect(14, y - 4, 4, 4, "F");
      doc.setTextColor(0);
      doc.text(`${sev.charAt(0).toUpperCase() + sev.slice(1)}: ${counts[sev]}`, 20, y);
      y += 6;
    });
    y += 6;

    // Separator line
    doc.setDrawColor(200);
    doc.line(14, y, 196, y);
    y += 10;

    addFooter();

    // Findings grouped by severity
    severities.forEach((sev) => {
      const sevFindings = findings.filter((f) => f.severity === sev);
      if (sevFindings.length === 0) return;

      // Check if we need a new page
      if (y > CONTENT_HEIGHT - 20) {
        doc.addPage();
        pageNum++;
        y = MARGIN_TOP;
        addHeader();
        y = 28;
        addFooter();
      }

      // Section header with severity color
      const [r, g, b] = SEVERITY_COLORS[sev];
      doc.setFillColor(r, g, b);
      doc.rect(14, y - 5, 4, 6, "F");
      
      doc.setFontSize(13);
      doc.setFont("helvetica", "bold");
      doc.setTextColor(0);
      doc.text(
        `${sev.charAt(0).toUpperCase() + sev.slice(1)} (${sevFindings.length})`,
        20,
        y
      );
      y += 8;

      sevFindings.forEach((f, i) => {
        // Calculate space needed for this finding
        const titleHeight = 6;
        const metadataHeight = 10;
        const snippetHeight = f.snippet ? 8 : 0;
        const suggestionHeight = f.suggestion ? 8 : 0;
        const spacingHeight = 6;
        const totalHeight = titleHeight + metadataHeight + snippetHeight + suggestionHeight + spacingHeight;

        // Check if we need a new page
        if (y + totalHeight > CONTENT_HEIGHT) {
          doc.addPage();
          pageNum++;
          y = MARGIN_TOP;
          addHeader();
          y = 28;
          addFooter();
        }

        doc.setFontSize(11);
        doc.setFont("helvetica", "bold");
        doc.setTextColor(0);
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
