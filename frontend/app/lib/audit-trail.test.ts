import { describe, it, expect, beforeEach, vi } from "vitest";
import {
  recordScanAudit,
  getAuditLog,
  clearAuditLog,
  type ScanAuditEntry,
} from "./audit-trail";

function makeEntry(overrides: Partial<ScanAuditEntry> = {}): ScanAuditEntry {
  return {
    timestamp: new Date().toISOString(),
    clientIp: "127.0.0.1",
    fileName: "contract.rs",
    fileSizeBytes: 1024,
    latencyMs: 120,
    outcomeCode: 200,
    totalFindings: 3,
    hasCritical: false,
    hasHigh: true,
    ...overrides,
  };
}

beforeEach(() => {
  clearAuditLog();
  vi.restoreAllMocks();
});

describe("recordScanAudit", () => {
  it("appends an entry to the audit log", () => {
    recordScanAudit(makeEntry());
    expect(getAuditLog()).toHaveLength(1);
  });

  it("records all fields verbatim", () => {
    const entry = makeEntry({ clientIp: "10.0.0.2", totalFindings: 7, hasCritical: true });
    recordScanAudit(entry);
    expect(getAuditLog()[0]).toEqual(entry);
  });

  it("emits a structured JSON log line via console.log", () => {
    const spy = vi.spyOn(console, "log").mockImplementation(() => {});
    const entry = makeEntry();
    recordScanAudit(entry);
    expect(spy).toHaveBeenCalledOnce();
    const [line] = spy.mock.calls[0];
    const parsed = JSON.parse(line as string);
    expect(parsed.event).toBe("scan_audit");
    expect(parsed.clientIp).toBe(entry.clientIp);
  });

  it("enforces the 1000-entry ring buffer cap", () => {
    for (let i = 0; i < 1_001; i++) recordScanAudit(makeEntry({ fileName: `f${i}.rs` }));
    const log = getAuditLog();
    expect(log).toHaveLength(1_000);
    // Oldest entry (f0) is evicted; most recent (f1000) is present.
    expect(log[log.length - 1].fileName).toBe("f1000.rs");
    expect(log[0].fileName).toBe("f1.rs");
  });
});

describe("getAuditLog", () => {
  it("returns an empty array before any scans", () => {
    expect(getAuditLog()).toHaveLength(0);
  });

  it("returns entries in insertion order", () => {
    recordScanAudit(makeEntry({ fileName: "a.rs" }));
    recordScanAudit(makeEntry({ fileName: "b.rs" }));
    const log = getAuditLog();
    expect(log[0].fileName).toBe("a.rs");
    expect(log[1].fileName).toBe("b.rs");
  });
});

describe("clearAuditLog", () => {
  it("empties the log", () => {
    recordScanAudit(makeEntry());
    recordScanAudit(makeEntry());
    clearAuditLog();
    expect(getAuditLog()).toHaveLength(0);
  });
});
