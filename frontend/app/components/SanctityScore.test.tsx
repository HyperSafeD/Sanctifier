import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { SanctityScore } from "./SanctityScore";
import type { Finding } from "../types";

const makeFinding = (severity: Finding["severity"]): Finding => ({
  id: `f-${severity}`,
  code: "TEST",
  severity,
  category: "Test",
  title: "Test finding",
  location: "test.rs:1",
  raw: null,
});

describe("SanctityScore", () => {
  it("renders a perfect score for an empty findings list", () => {
    render(<SanctityScore findings={[]} />);

    const svg = screen.getByRole("img");
    expect(svg).toHaveAttribute(
      "aria-label",
      expect.stringContaining("100")
    );
  });

  it("reduces score based on finding severities", () => {
    const findings = [makeFinding("critical"), makeFinding("high")];
    render(<SanctityScore findings={findings} />);

    // critical=15 + high=10 → score = 75
    const svg = screen.getByRole("img");
    expect(svg).toHaveAttribute(
      "aria-label",
      expect.stringContaining("75")
    );
  });

  it("displays the correct grade label", () => {
    render(<SanctityScore findings={[]} />);

    expect(screen.getByText("Grade: A")).toBeInTheDocument();
  });
});
