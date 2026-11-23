import { describe, it, expect } from "vitest";
import { recentScanRecords } from "./trend";
import type { ScanRecord } from "./scan-history";

const DAY = 24 * 60 * 60 * 1000;
const NOW = Date.parse("2026-09-26T00:00:00Z");
const rec = (daysAgo: number): ScanRecord => ({
  workspace: "w",
  timestamp: NOW - daysAgo * DAY,
  critical: 1,
  high: 2,
  medium: 3,
  low: 4,
  total: 10,
});

describe("recentScanRecords", () => {
  it("keeps only the last 30 days, oldest first", () => {
    const out = recentScanRecords([rec(1), rec(45), rec(29), rec(31), rec(10)], 30, NOW);
    expect(out.map((r) => Math.round((NOW - r.timestamp) / DAY))).toEqual([29, 10, 1]);
  });

  it("includes a record exactly on the cutoff and returns [] for empty input", () => {
    expect(recentScanRecords([rec(30)], 30, NOW)).toHaveLength(1);
    expect(recentScanRecords([], 30, NOW)).toEqual([]);
  });
});
