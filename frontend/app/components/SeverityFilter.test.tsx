import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { SeverityFilter } from "./SeverityFilter";

describe("SeverityFilter", () => {
  it("renders all filter buttons", () => {
    render(<SeverityFilter selected="all" onChange={() => {}} />);

    expect(screen.getByText("All")).toBeInTheDocument();
    expect(screen.getByText("Critical")).toBeInTheDocument();
    expect(screen.getByText("High")).toBeInTheDocument();
    expect(screen.getByText("Medium")).toBeInTheDocument();
    expect(screen.getByText("Low")).toBeInTheDocument();
  });

  it("marks the selected filter as pressed", () => {
    render(<SeverityFilter selected="high" onChange={() => {}} />);

    expect(screen.getByText("High")).toHaveAttribute("aria-pressed", "true");
    expect(screen.getByText("All")).toHaveAttribute("aria-pressed", "false");
  });

  it("calls onChange when a filter button is clicked", async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();

    render(<SeverityFilter selected="all" onChange={onChange} />);

    await user.click(screen.getByText("Critical"));
    expect(onChange).toHaveBeenCalledWith("critical");
  });
});
