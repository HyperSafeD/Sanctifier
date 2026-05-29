import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { POST } from "./route";
import { NextRequest } from "next/server";
import * as fs from "fs/promises";
import * as childProcess from "child_process";

// Mock modules
vi.mock("fs/promises");
vi.mock("child_process");
vi.mock("../../lib/transform", () => ({
  normalizeReport: vi.fn((report) => report),
  transformReport: vi.fn((report) => []),
}));
vi.mock("../../lib/audit-trail", () => ({
  recordScanAudit: vi.fn(),
}));

describe("/api/analyze POST - temp dir cleanup", () => {
  let mockTempDir: string;

  beforeEach(() => {
    mockTempDir = "/tmp/sanctifier-contract-test123";
    vi.clearAllMocks();

    // Mock mkdtemp to return a test directory
    vi.mocked(fs.mkdtemp).mockResolvedValue(mockTempDir);
    vi.mocked(fs.writeFile).mockResolvedValue(undefined);
    vi.mocked(fs.rm).mockResolvedValue(undefined);
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("should clean up temp dir on successful analysis", async () => {
    // Mock successful CLI execution
    const mockSpawn = vi.fn().mockReturnValue({
      stdout: { on: vi.fn((event, cb) => event === "data" && cb('{"findings":[]}')) },
      stderr: { on: vi.fn() },
      on: vi.fn((event, cb) => event === "close" && cb(0)),
    });
    vi.mocked(childProcess.spawn).mockImplementation(mockSpawn as any);

    const request = new NextRequest("http://localhost:3000/api/analyze", {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ source: "use soroban_sdk;\nfn test() {}" }),
    });

    await POST(request);

    expect(fs.rm).toHaveBeenCalledWith(mockTempDir, { recursive: true, force: true });
  });

  it("should clean up temp dir when CLI times out", async () => {
    // Mock CLI that never completes (simulates timeout)
    const mockSpawn = vi.fn().mockReturnValue({
      stdout: { on: vi.fn() },
      stderr: { on: vi.fn() },
      on: vi.fn(), // Never calls close callback
      kill: vi.fn(),
    });
    vi.mocked(childProcess.spawn).mockImplementation(mockSpawn as any);

    // Mock timeout by making the promise reject
    vi.spyOn(global, "setTimeout").mockImplementation(((cb: () => void) => {
      cb(); // Immediately trigger timeout
      return 1 as any;
    }) as any);

    const request = new NextRequest("http://localhost:3000/api/analyze", {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ source: "use soroban_sdk;\nfn test() {}" }),
    });

    await POST(request);

    // Verify temp dir was cleaned up despite timeout
    expect(fs.rm).toHaveBeenCalledWith(mockTempDir, { recursive: true, force: true });
  });

  it("should clean up temp dir when CLI throws error", async () => {
    // Mock CLI that errors
    const mockSpawn = vi.fn().mockReturnValue({
      stdout: { on: vi.fn() },
      stderr: { on: vi.fn() },
      on: vi.fn((event, cb) => {
        if (event === "error") {
          cb(new Error("CLI execution failed"));
        }
      }),
    });
    vi.mocked(childProcess.spawn).mockImplementation(mockSpawn as any);

    const request = new NextRequest("http://localhost:3000/api/analyze", {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ source: "use soroban_sdk;\nfn test() {}" }),
    });

    await POST(request);

    // Verify temp dir was cleaned up despite error
    expect(fs.rm).toHaveBeenCalledWith(mockTempDir, { recursive: true, force: true });
  });

  it("should clean up temp dir when file write fails", async () => {
    // Mock writeFile to fail
    vi.mocked(fs.writeFile).mockRejectedValue(new Error("Disk full"));

    const request = new NextRequest("http://localhost:3000/api/analyze", {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ source: "use soroban_sdk;\nfn test() {}" }),
    });

    await POST(request);

    // Verify temp dir was cleaned up despite write failure
    expect(fs.rm).toHaveBeenCalledWith(mockTempDir, { recursive: true, force: true });
  });

  it("should handle rm failure gracefully", async () => {
    // Mock rm to fail (e.g., permission denied)
    vi.mocked(fs.rm).mockRejectedValue(new Error("Permission denied"));

    const mockSpawn = vi.fn().mockReturnValue({
      stdout: { on: vi.fn((event, cb) => event === "data" && cb('{"findings":[]}')) },
      stderr: { on: vi.fn() },
      on: vi.fn((event, cb) => event === "close" && cb(0)),
    });
    vi.mocked(childProcess.spawn).mockImplementation(mockSpawn as any);

    const request = new NextRequest("http://localhost:3000/api/analyze", {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ source: "use soroban_sdk;\nfn test() {}" }),
    });

    // Should not throw even if rm fails
    await expect(POST(request)).resolves.toBeDefined();
  });
});
