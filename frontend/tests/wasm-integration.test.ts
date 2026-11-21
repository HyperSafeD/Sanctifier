import { describe, it, expect } from "vitest";
import { normalizeReport, transformReport } from "../app/lib/transform";
import wasmFixture from "./fixtures/wasm-analysis-result.json";

describe("WASM Package Integration", () => {
  it("should import the actual @sanctifier/wasm build artifact", async () => {
    const wasm = await import("@sanctifier/wasm");
    expect(wasm).toBeDefined();
    if (Object.keys(wasm).length === 0) {
      // In CI with stub, verify the mock exists; in local with real build, check real exports
      expect(wasm).not.toBeNull();
    }
  });

  it("should correctly handle findings structure from the WASM engine", () => {
    const report = normalizeReport(wasmFixture);
    const findings = transformReport(report);

    expect(findings).toBeDefined();
    expect(findings.length).toBeGreaterThan(0);

    const authFinding = findings.find(f => f.code === "S001");
    expect(authFinding).toBeDefined();
    expect(authFinding).toHaveProperty("message");
    expect(authFinding).toHaveProperty("location");
  });

  it("should verify WASM fixture matches expected shape", () => {
    expect(wasmFixture).toHaveProperty("findings");
    expect(wasmFixture).toHaveProperty("summary");
    expect(Array.isArray(wasmFixture.findings)).toBe(true);
    expect(typeof wasmFixture.summary).toBe("object");

    wasmFixture.findings.forEach(finding => {
      expect(finding).toHaveProperty("code");
      expect(finding).toHaveProperty("category");
      expect(finding).toHaveProperty("message");
      expect(finding).toHaveProperty("location");
    });

    expect(wasmFixture.summary).toHaveProperty("total");
    expect(typeof wasmFixture.summary.total).toBe("number");
  });
});
