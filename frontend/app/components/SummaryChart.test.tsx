import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { SummaryChart } from "./SummaryChart";
import type { Finding } from "../types";

const makeFinding = (severity: Finding["severity"]): Finding => ({
  id: `f-${severity}-${Math.random()}`,
  code: "TEST",
  severity,
  category: "Test",
  title: "Test finding",
  location: "test.rs:1",
  raw: null,
});

describe("SummaryChart", () => {
  it("renders bars for each severity level", () => {
    const findings = [
      makeFinding("critical"),
      makeFinding("high"),
      makeFinding("medium"),
      makeFinding("low"),
    ];
    render(<SummaryChart findings={findings} />);

    expect(screen.getByText("critical")).toBeInTheDocument();
    expect(screen.getByText("high")).toBeInTheDocument();
    expect(screen.getByText("medium")).toBeInTheDocument();
    expect(screen.getByText("low")).toBeInTheDocument();
  });

  it("shows total findings count", () => {
    const findings = [makeFinding("critical"), makeFinding("high")];
    render(<SummaryChart findings={findings} />);

    expect(screen.getByText("Total: 2 findings")).toBeInTheDocument();
  });

  it("renders zero counts when no findings exist", () => {
    render(<SummaryChart findings={[]} />);

    expect(screen.getByText("Total: 0 findings")).toBeInTheDocument();
    expect(screen.getAllByText("0")).toHaveLength(4);
  });
});
