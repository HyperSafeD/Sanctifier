import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { ThemeToggle } from "./ThemeToggle";

const mockToggle = vi.fn();
let currentTheme = "light";

vi.mock("../providers/theme-provider", () => ({
  useTheme: () => ({
    theme: currentTheme,
    toggleTheme: mockToggle,
    setTheme: vi.fn(),
  }),
}));

describe("ThemeToggle", () => {
  it("renders with correct label for light mode", () => {
    currentTheme = "light";
    render(<ThemeToggle />);

    expect(screen.getByText("Switch to Dark")).toBeInTheDocument();
  });

  it("renders with correct label for dark mode", () => {
    currentTheme = "dark";
    render(<ThemeToggle />);

    expect(screen.getByText("Switch to Light")).toBeInTheDocument();
  });

  it("calls toggleTheme on click", async () => {
    currentTheme = "light";
    const user = userEvent.setup();
    render(<ThemeToggle />);

    await user.click(screen.getByRole("button"));
    expect(mockToggle).toHaveBeenCalledOnce();
  });
});
