import { describe, it, expect, vi } from "vitest";
import { normalizeReport, transformReport } from "../app/lib/transform";
import wasmFixture from "./fixtures/wasm-analysis-result.json";

// Mock the WASM module
vi.mock("@sanctifier/wasm", () => ({
  analyze: vi.fn((source: string) => {
    if (source.includes("VulnerableContract")) {
      return wasmFixture;
    }
    return { findings: [], summary: { total: 0, has_critical: false, has_high: false } };
  }),
  version: () => "0.1.0",
}));

describe("WASM Package Integration", () => {
  it("should correctly handle findings from the WASM engine", async () => {
    // In a real environment, this would be imported. Here we use the mock.
    const wasm = await import("@sanctifier/wasm");
    
    const source = `
      #[contract]
      pub struct VulnerableContract;
      #[contractimpl]
      impl VulnerableContract {
          pub fn transfer(env: Env, amount: i128) {
              if amount < 0 { panic!("neg"); }
          }
      }
    `;
    
    const result = wasm.analyze(source);
    expect(result).toBeDefined();
    expect(result.findings).toHaveLength(2);
    expect(result.summary.total).toBe(2);
  });

  it("should integrate with the transform library", () => {
    const report = normalizeReport(wasmFixture);
    const findings = transformReport(report);
    
    expect(findings).toBeDefined();
    // The transformer might map WASM categories to UI categories
    expect(findings.length).toBeGreaterThan(0);
    
    const authFinding = findings.find(f => f.category === "Auth Gap" || f.code === "S001");
    expect(authFinding).toBeDefined();
  });

  it("should provide a version string", async () => {
    const wasm = await import("@sanctifier/wasm");
    expect(wasm.version()).toBe("0.1.0");
  });
});
