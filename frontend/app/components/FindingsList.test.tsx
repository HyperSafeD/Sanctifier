import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { FindingsList } from "./FindingsList";
import { createFinding, createFindingList } from "../../tests/fixtures";

describe("FindingsList", () => {
  it("renders findings", () => {
    const findings = [
      createFinding({ title: "Auth Gap", severity: "critical" }),
      createFinding({ title: "Panic Usage", severity: "high" }),
    ];

    render(
      <FindingsList findings={findings} severityFilter="all" codeFilter="" />,
    );

    expect(screen.getByText("Auth Gap")).toBeInTheDocument();
    expect(screen.getByText("Panic Usage")).toBeInTheDocument();
  });

  it("shows empty state when no findings match filter", () => {
    const findings = [createFinding({ code: "S001", severity: "critical" })];

    render(
      <FindingsList
        findings={findings}
        severityFilter="critical"
        codeFilter="S999"
      />,
    );

    expect(
      screen.getByText("No findings match the selected filter."),
    ).toBeInTheDocument();
  });

  it("filters findings by severity", () => {
    const findings = [
      createFinding({ title: "Critical Issue", severity: "critical" }),
      createFinding({ title: "High Issue", severity: "high" }),
    ];

    render(
      <FindingsList
        findings={findings}
        severityFilter="critical"
        codeFilter=""
      />,
    );

    expect(screen.getByText("Critical Issue")).toBeInTheDocument();
    expect(screen.queryByText("High Issue")).not.toBeInTheDocument();
  });

  it("filters findings by code", () => {
    const findings = [
      createFinding({ code: "S001", title: "Finding 1" }),
      createFinding({ code: "S002", title: "Finding 2" }),
    ];

    render(
      <FindingsList
        findings={findings}
        severityFilter="all"
        codeFilter="S001"
      />,
    );

    expect(screen.getByText("Finding 1")).toBeInTheDocument();
    expect(screen.queryByText("Finding 2")).not.toBeInTheDocument();
  });

  it("renders small lists without virtualization", () => {
    const findings = createFindingList(10);

    render(
      <FindingsList findings={findings} severityFilter="all" codeFilter="" />,
    );

    // All items should be rendered
    findings.forEach((f) => {
      expect(screen.getByText(f.title)).toBeInTheDocument();
    });
  });

  it("handles empty findings array", () => {
    render(<FindingsList findings={[]} severityFilter="all" codeFilter="" />);

    expect(
      screen.getByText("No findings match the selected filter."),
    ).toBeInTheDocument();
  });

  it("updates when filter changes", () => {
    const findings = [
      createFinding({ title: "Critical", severity: "critical" }),
      createFinding({ title: "High", severity: "high" }),
    ];

    const { rerender } = render(
      <FindingsList findings={findings} severityFilter="all" codeFilter="" />,
    );

    expect(screen.getByText("Critical")).toBeInTheDocument();
    expect(screen.getByText("High")).toBeInTheDocument();

    rerender(
      <FindingsList
        findings={findings}
        severityFilter="critical"
        codeFilter=""
      />,
    );

    expect(screen.getByText("Critical")).toBeInTheDocument();
    expect(screen.queryByText("High")).not.toBeInTheDocument();
  });

  it("has proper accessibility attributes", () => {
    const findings = [createFinding({ severity: "critical", title: "Test" })];

    render(
      <FindingsList findings={findings} severityFilter="all" codeFilter="" />,
    );

    const severityBadge = screen.getByLabelText("Critical severity");
    expect(severityBadge).toBeInTheDocument();
  });
});
