import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { TrendPanel } from "./TrendPanel";
import type { ScanRecord } from "../lib/scan-history";

const rec = (daysAgo: number, critical: number): ScanRecord => ({
  workspace: "w",
  timestamp: Date.now() - daysAgo * 86_400_000,
  critical,
  high: 2,
  medium: 3,
  low: 4,
  total: critical + 9,
});

describe("TrendPanel", () => {
  it("asks for more scans when there is not enough history", () => {
    render(<TrendPanel records={[rec(1, 1)]} />);
    expect(screen.getByText(/Run at least two scans/)).toBeInTheDocument();
  });

  it("ignores scans older than 30 days", () => {
    render(<TrendPanel records={[rec(40, 5), rec(35, 6)]} />);
    expect(screen.getByText(/Run at least two scans/)).toBeInTheDocument();
  });

  it("shows a chart per severity with the latest count", () => {
    render(<TrendPanel records={[rec(5, 1), rec(1, 7)]} />);
    for (const label of ["Critical: 7", "High: 2", "Medium: 3", "Low: 4"]) {
      expect(screen.getByText(label)).toBeInTheDocument();
    }
    expect(screen.getByRole("heading", { name: /last 30 days/ })).toBeInTheDocument();
  });
});
