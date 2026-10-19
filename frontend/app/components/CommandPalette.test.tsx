import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

const push = vi.fn();
vi.mock("next/navigation", () => ({ useRouter: () => ({ push }) }));
vi.mock("../providers/theme-provider", () => ({ useTheme: () => ({ toggleTheme: vi.fn() }) }));

import { CommandPalette } from "./CommandPalette";

describe("CommandPalette", () => {
  beforeEach(() => {
    push.mockClear();
    window.localStorage.clear();
    // Never resolves, so no state update lands outside act() after a test finishes.
    vi.stubGlobal("fetch", vi.fn(() => new Promise(() => {})));
  });

  it("is closed until Ctrl+K is pressed, then opens with the input focused", () => {
    render(<CommandPalette />);
    expect(screen.queryByRole("dialog")).toBeNull();

    fireEvent.keyDown(window, { key: "k", ctrlKey: true });

    expect(screen.getByRole("dialog")).toBeInTheDocument();
    expect(screen.getByRole("combobox")).toHaveFocus();
  });

  it("also opens with ⌘K and closes on Escape", () => {
    render(<CommandPalette />);
    fireEvent.keyDown(window, { key: "k", metaKey: true });
    expect(screen.getByRole("dialog")).toBeInTheDocument();

    fireEvent.keyDown(screen.getByRole("combobox"), { key: "Escape" });
    expect(screen.queryByRole("dialog")).toBeNull();
  });

  it("filters results as the user types and navigates with arrows + Enter", async () => {
    const user = userEvent.setup();
    render(<CommandPalette />);
    fireEvent.keyDown(window, { key: "k", ctrlKey: true });

    await user.type(screen.getByRole("combobox"), "a");
    const options = screen.getAllByRole("option");
    expect(options.length).toBeGreaterThan(1);

    await user.keyboard("{ArrowDown}");
    expect(screen.getAllByRole("option")[1]).toHaveAttribute("aria-selected", "true");

    await user.keyboard("{Enter}");
    expect(push).toHaveBeenCalledTimes(1);
    expect(screen.queryByRole("dialog")).toBeNull();
  });

  it("shows recent items by default after a selection", async () => {
    const user = userEvent.setup();
    render(<CommandPalette />);
    fireEvent.keyDown(window, { key: "k", ctrlKey: true });
    await user.type(screen.getByRole("combobox"), "audit");
    await user.keyboard("{Enter}");
    expect(push).toHaveBeenCalledWith("/audit");

    fireEvent.keyDown(window, { key: "k", ctrlKey: true });
    expect(screen.getByText("Recent")).toBeInTheDocument();
    expect(screen.getAllByRole("option")[0]).toHaveTextContent("Audit Report");
  });

  it("renders above other content (z-index ≥ 50)", () => {
    const { container } = render(<CommandPalette />);
    fireEvent.keyDown(window, { key: "k", ctrlKey: true });
    expect(container.firstElementChild?.className).toMatch(/z-\[100\]/);
  });
});
