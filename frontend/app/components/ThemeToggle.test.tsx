import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { ThemeToggle } from "./ThemeToggle";

const mockSetTheme = vi.fn();
let currentTheme = "light";

vi.mock("../providers/theme-provider", () => ({
  useTheme: () => ({
    theme: currentTheme,
    setTheme: mockSetTheme,
    toggleTheme: vi.fn(),
  }),
}));

describe("ThemeToggle", () => {
  beforeEach(() => {
    mockSetTheme.mockClear();
  });

  it("renders with correct label for light mode", () => {
    currentTheme = "light";
    render(<ThemeToggle />);

    expect(screen.getByRole("button", { name: "Light" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Dark" })).toBeInTheDocument();
  });

  it("renders with correct label for dark mode", () => {
    currentTheme = "dark";
    render(<ThemeToggle />);

    expect(screen.getByRole("button", { name: "Dark" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Light" })).toBeInTheDocument();
  });

  it("calls setTheme on click", async () => {
    currentTheme = "light";
    const user = userEvent.setup();
    render(<ThemeToggle />);

    await user.click(screen.getByRole("button", { name: "Dark" }));
    expect(mockSetTheme).toHaveBeenCalledWith("dark");
  });
});
