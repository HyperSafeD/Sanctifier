import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { SummaryChart } from "./SummaryChart";
import { createFinding, createFindingList } from "../../tests/fixtures";

describe("SummaryChart", () => {
  it("renders bars for each severity level", () => {
    const findings = [
      createFinding({ severity: "critical" }),
      createFinding({ severity: "high" }),
      createFinding({ severity: "medium" }),
      createFinding({ severity: "low" }),
    ];
    render(<SummaryChart findings={findings} />);

    expect(screen.getByText("critical")).toBeInTheDocument();
    expect(screen.getByText("high")).toBeInTheDocument();
    expect(screen.getByText("medium")).toBeInTheDocument();
    expect(screen.getByText("low")).toBeInTheDocument();
  });

  it("shows total findings count", () => {
    const findings = [
      createFinding({ severity: "critical" }),
      createFinding({ severity: "high" }),
    ];
    render(<SummaryChart findings={findings} />);

    expect(screen.getByText(/Total: 2 findings/)).toBeInTheDocument();
  });

  it("renders zero counts when no findings exist", () => {
    render(<SummaryChart findings={[]} />);

    expect(screen.getByText(/Total: 0 findings/)).toBeInTheDocument();
    expect(screen.getAllByText("0")).toHaveLength(4);
  });

  it("calculates correct counts for multiple findings", () => {
    const findings = [
      ...createFindingList(5, "critical"),
      ...createFindingList(3, "high"),
      ...createFindingList(2, "medium"),
      ...createFindingList(1, "low"),
    ];
    render(<SummaryChart findings={findings} />);

    expect(screen.getByText(/Total: 11 findings/)).toBeInTheDocument();
  });

  it("renders title", () => {
    render(<SummaryChart findings={[]} />);
    expect(screen.getByText("Findings by Severity")).toBeInTheDocument();
  });

  it("handles single finding", () => {
    const findings = [createFinding({ severity: "critical" })];
    render(<SummaryChart findings={findings} />);

    expect(screen.getByText(/Total: 1 findings/)).toBeInTheDocument();
  });
});
