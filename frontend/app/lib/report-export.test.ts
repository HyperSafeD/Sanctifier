import { describe, expect, it } from "vitest";
import type { Finding } from "../types";
import {
  buildAuditExportModel,
  buildCsvContent,
  calculateScore,
  getScoreGrade,
} from "./report-export";

const findings: Finding[] = [
  {
    id: "1",
    code: "S001",
    severity: "critical",
    category: "Auth Gap",
    title: "Missing require_auth()",
    location: "src/lib.rs:10",
    suggestion: "Add require_auth().",
    raw: null,
  },
  {
    id: "2",
    code: "S008",
    severity: "medium",
    category: "Event Issue",
    title: "Inconsistent topics",
    location: "src/lib.rs:24",
    snippet: "publish((symbol_short!(\"evt\"),), data);",
    raw: null,
  },
];

describe("report export helpers", () => {
  it("builds an audit model with score and severity buckets", () => {
    const model = buildAuditExportModel(findings, null);

    expect(model.score).toBe(80);
    expect(model.grade).toBe("B");
    expect(model.severityCounts.critical).toBe(1);
    expect(model.severityCounts.medium).toBe(1);
    expect(model.findingsBySeverity.critical).toHaveLength(1);
  });

  it("builds CSV content with quoted cells", () => {
    const csv = buildCsvContent(findings, null);

    expect(csv).toContain('"report_title","generated_at","sanctity_score"');
    expect(csv).toContain('"Missing require_auth()"');
    expect(csv).toContain('"Add require_auth()."');
  });

  it("shares score helpers with the dashboard", () => {
    expect(calculateScore(findings)).toBe(80);
    expect(getScoreGrade(80)).toBe("B");
  });
});
