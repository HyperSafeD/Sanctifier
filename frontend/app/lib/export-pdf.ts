import type { Finding } from "../types";

export async function exportToPdf(
  findings: Finding[],
  title = "Sanctifier Security Report"
): Promise<void> {
  try {
    const { jsPDF } = await import("jspdf");
    const doc = new jsPDF();

    doc.setFontSize(18);
    doc.text(title, 14, 22);

    doc.setFontSize(10);
    doc.text(`Generated: ${new Date().toLocaleString()}`, 14, 30);
    doc.text(`Total findings: ${findings.length}`, 14, 36);

    let y = 50;

    findings.forEach((f, i) => {
      if (y > 270) {
        doc.addPage();
        y = 20;
      }

      doc.setFontSize(12);
      doc.setFont("helvetica", "bold");
      doc.text(`${i + 1}. [${f.severity.toUpperCase()}] ${f.title}`, 14, y);
      y += 6;

      doc.setFont("helvetica", "normal");
      doc.setFontSize(10);
      doc.text(`Category: ${f.category}`, 14, y);
      y += 5;
      doc.text(`Location: ${f.location}`, 14, y);
      y += 5;

      if (f.snippet) {
        const snippetLines = doc.splitTextToSize(f.snippet, 180);
        doc.text(snippetLines, 14, y);
        y += snippetLines.length * 5;
      }
      if (f.suggestion) {
        doc.text(`Suggestion: ${f.suggestion}`, 14, y);
        y += 5;
      }
      y += 8;
    });

    doc.save("sanctifier-report.pdf");
  } catch {
    window.print();
  }
}
