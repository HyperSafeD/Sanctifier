import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { Skeleton, FindingsListSkeleton, DashboardCardsSkeleton, ContractCardSkeleton } from "./Skeleton";

describe("Skeleton", () => {
  it("applies width, height and rounding and pulses", () => {
    render(<Skeleton width="60%" height={24} rounded="full" />);
    const el = screen.getByTestId("skeleton");
    expect(el).toHaveStyle({ width: "60%", height: "24px" });
    expect(el.className).toContain("animate-pulse");
    expect(el.className).toContain("rounded-full");
    expect(el).toHaveAttribute("aria-hidden", "true");
  });

  it("renders the requested number of findings rows in a busy status region", () => {
    render(<FindingsListSkeleton rows={3} />);
    const region = screen.getByRole("status", { name: "Loading findings" });
    expect(region).toHaveAttribute("aria-busy", "true");
    expect(region.querySelectorAll(".animate-pulse").length).toBeGreaterThanOrEqual(3);
  });

  it("renders dashboard and contract card placeholders", () => {
    const { container } = render(
      <>
        <DashboardCardsSkeleton />
        <ContractCardSkeleton />
      </>,
    );
    expect(screen.getByRole("status", { name: "Loading dashboard" })).toBeInTheDocument();
    expect(container.querySelectorAll(".animate-pulse").length).toBeGreaterThan(5);
  });
});
