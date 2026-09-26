import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";

let mockPath = "/contracts/governance";
vi.mock("next/navigation", () => ({ usePathname: () => mockPath }));

import { Breadcrumbs } from "./Breadcrumbs";

describe("Breadcrumbs", () => {
  it("renders the trail from the URL with the last crumb as the current page", () => {
    mockPath = "/contracts/governance";
    render(<Breadcrumbs />);

    expect(screen.getByRole("navigation", { name: "Breadcrumb" })).toBeInTheDocument();
    expect(screen.getByRole("link", { name: "Home" })).toHaveAttribute("href", "/");
    expect(screen.getByRole("link", { name: "Contracts" })).toHaveAttribute("href", "/contracts");
    expect(screen.getByText("governance")).toHaveAttribute("aria-current", "page");
  });

  it("renders nothing on the home page", () => {
    mockPath = "/";
    const { container } = render(<Breadcrumbs />);
    expect(container).toBeEmptyDOMElement();
  });

  it("accepts explicit items", () => {
    render(<Breadcrumbs items={[{ label: "A", href: "/a" }, { label: "B", href: "/a/b" }]} />);
    expect(screen.getByRole("link", { name: "A" })).toBeInTheDocument();
    expect(screen.getByText("B")).toHaveAttribute("aria-current", "page");
  });
});
