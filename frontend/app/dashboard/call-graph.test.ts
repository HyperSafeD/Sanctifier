/**
 * Regression tests for extractCallGraph shape (issue #858).
 *
 * Root cause: dashboard/page.tsx destructured { callGraphNodes, callGraphEdges }
 * but extractCallGraph() returns { nodes, edges }. This test suite ensures the
 * return shape is statically and dynamically correct.
 */
import { describe, it, expect } from "vitest";
import { extractCallGraph } from "../lib/transform";
import type { AnalysisReport } from "../types";

function makeReport(overrides: Partial<AnalysisReport> = {}): AnalysisReport {
  return {
    contract_name: "TestContract",
    file_path: "src/lib.rs",
    findings: [],
    auth_gaps: [],
    panic_risks: [],
    storage_patterns: [],
    call_graph: [],
    ...overrides,
  } as AnalysisReport;
}

describe("extractCallGraph", () => {
  it("returns { nodes, edges } — not { callGraphNodes, callGraphEdges }", () => {
    const result = extractCallGraph(makeReport());
    // Ensure the correct keys exist
    expect(result).toHaveProperty("nodes");
    expect(result).toHaveProperty("edges");
    // Ensure the old (wrong) keys do NOT exist
    expect(result).not.toHaveProperty("callGraphNodes");
    expect(result).not.toHaveProperty("callGraphEdges");
  });

  it("returns empty arrays for a report with no findings", () => {
    const { nodes, edges } = extractCallGraph(makeReport());
    expect(Array.isArray(nodes)).toBe(true);
    expect(Array.isArray(edges)).toBe(true);
  });

  it("extracts nodes from auth_gaps", () => {
    const report = makeReport({
      auth_gaps: ["src/lib.rs:transfer", "src/lib.rs:withdraw"],
    });
    const { nodes } = extractCallGraph(report);
    expect(nodes.length).toBeGreaterThan(0);
    const labels = nodes.map((n) => n.label);
    expect(labels).toContain("transfer");
    expect(labels).toContain("withdraw");
  });

  it("extracts nodes from call_graph when present", () => {
    const report = makeReport({
      call_graph: [
        { caller: "fn_a", callee: "fn_b", file: "src/lib.rs" },
      ] as any,
    });
    const { nodes, edges } = extractCallGraph(report);
    expect(nodes.length).toBeGreaterThan(0);
    expect(edges.length).toBeGreaterThan(0);
  });

  it("dashboard memo destructuring uses correct keys", () => {
    // This is a compile-time guard expressed as a runtime check.
    // If the destructuring in dashboard/page.tsx ever reverts to the wrong
    // key names, this test will still pass — but the TypeScript compiler
    // will catch it via the ReturnType<> annotation already in place.
    const report = makeReport({ auth_gaps: ["src/lib.rs:do_work"] });
    const graph = extractCallGraph(report);
    // Simulate the dashboard memo destructuring
    const { nodes: callGraphNodes, edges: callGraphEdges } = graph;
    expect(callGraphNodes).toBeDefined();
    expect(callGraphEdges).toBeDefined();
  });
});
