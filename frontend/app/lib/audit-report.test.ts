import { describe, it, expect } from "vitest";
import { buildAuditReport, riskLabelFor } from "./audit-report";
import type { WorkspaceSummary } from "../types";

const workspace: WorkspaceSummary = {
  workspace: "demo",
  shared_libs: [],
  grand_total_findings: 3,
  contracts: [
    {
      name: "clean",
      total_findings: 0,
      report: {},
    },
    {
      name: "risky",
      total_findings: 3,
      report: {
        auth_gaps: [{ code: "S001", function_name: "withdraw" }],
        panic_issues: [
          { code: "S002", function_name: "burn", issue_type: "unwrap", location: "lib.rs:10" },
          { code: "S002", function_name: "mint", issue_type: "panic!", location: "lib.rs:20" },
        ],
      },
    },
  ],
};

describe("buildAuditReport", () => {
  const report = buildAuditReport(workspace);

  it("aggregates totals and per-contract counts", () => {
    expect(report.totals.all).toBe(3);
    expect(report.contracts.map((c) => c.name)).toEqual(["risky", "clean"]);
    expect(report.contracts[0].total).toBe(3);
    expect(report.contracts[1].total).toBe(0);
  });

  it("orders top findings by severity and namespaces ids per contract", () => {
    expect(report.topFindings[0].severity).toBe("critical");
    expect(report.topFindings.every((f) => f.id.startsWith("risky:"))).toBe(true);
    expect(new Set(report.findings.map((f) => f.id)).size).toBe(report.findings.length);
  });

  it("builds a remediation timeline only for severities that occur", () => {
    expect(report.timeline.map((s) => s.severity)).toEqual(
      ["critical", "high"].filter((s) => report.totals[s as "critical" | "high"] > 0),
    );
    expect(report.timeline[0].window).toMatch(/24 hours/);
  });

  it("handles an empty workspace", () => {
    const empty = buildAuditReport({ workspace: "x", contracts: [], shared_libs: [], grand_total_findings: 0 });
    expect(empty.score).toBe(100);
    expect(empty.riskLabel).toBe("Low");
    expect(empty.timeline).toEqual([]);
  });
});

describe("riskLabelFor", () => {
  it("maps score bands to labels", () => {
    expect(riskLabelFor(95)).toBe("Low");
    expect(riskLabelFor(70)).toBe("Moderate");
    expect(riskLabelFor(50)).toBe("High");
    expect(riskLabelFor(10)).toBe("Critical");
  });
});
