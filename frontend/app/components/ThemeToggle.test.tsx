import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { ThemeToggle } from "./ThemeToggle";

const mockSetTheme = vi.fn();
let currentTheme = "light";

vi.mock("../providers/theme-provider", () => ({
  useTheme: () => ({
    theme: currentTheme,
    setTheme: mockSetTheme,
  }),
}));

describe("ThemeToggle", () => {
  it("renders all theme options", () => {
    currentTheme = "light";
    render(<ThemeToggle />);

    expect(screen.getByRole("button", { name: "Light" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Dark" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "System" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "High Contrast" })).toBeInTheDocument();
  });

  it("indicates active theme via aria-pressed", () => {
    currentTheme = "dark";
    render(<ThemeToggle />);

    expect(screen.getByRole("button", { name: "Dark" })).toHaveAttribute("aria-pressed", "true");
    expect(screen.getByRole("button", { name: "Light" })).toHaveAttribute("aria-pressed", "false");
  });

  it("calls setTheme on click", async () => {
    currentTheme = "light";
    const user = userEvent.setup();
    render(<ThemeToggle />);

    await user.click(screen.getByRole("button", { name: "Dark" }));
    expect(mockSetTheme).toHaveBeenCalledWith("dark");
  });
});
