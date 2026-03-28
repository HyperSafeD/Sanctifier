import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { FindingsList } from "./FindingsList";
import type { Finding } from "../types";

const findings: Finding[] = [
  {
    id: "1",
    code: "S001",
    severity: "critical",
    category: "Auth Gap",
    title: "Missing require_auth()",
    location: "src/lib.rs:10",
    suggestion: "Add require_auth().",
    raw: null,
  },
  {
    id: "2",
    code: "S003",
    severity: "high",
    category: "Arithmetic",
    title: "Unchecked add",
    location: "src/lib.rs:20",
    raw: null,
  },
  {
    id: "3",
    code: "S005",
    severity: "medium",
    category: "Storage Collision",
    title: "Key collision",
    location: "src/lib.rs:30",
    raw: null,
  },
];

describe("FindingsList", () => {
  it("renders all findings when filter is 'all'", () => {
    render(<FindingsList findings={findings} severityFilter="all" />);

    expect(screen.getByText("Missing require_auth()")).toBeInTheDocument();
    expect(screen.getByText("Unchecked add")).toBeInTheDocument();
    expect(screen.getByText("Key collision")).toBeInTheDocument();
  });

  it("filters findings by severity", () => {
    render(<FindingsList findings={findings} severityFilter="critical" />);

    expect(screen.getByText("Missing require_auth()")).toBeInTheDocument();
    expect(screen.queryByText("Unchecked add")).not.toBeInTheDocument();
    expect(screen.queryByText("Key collision")).not.toBeInTheDocument();
  });

  it("shows empty state when no findings match", () => {
    render(<FindingsList findings={findings} severityFilter="low" />);

    expect(
      screen.getByText("No findings match the selected filter.")
    ).toBeInTheDocument();
  });

  it("displays suggestion when present", () => {
    render(<FindingsList findings={findings} severityFilter="all" />);

    expect(screen.getByText(/Add require_auth/)).toBeInTheDocument();
  });
});
