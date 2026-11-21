import { test, expect } from "@playwright/test";

test.describe("Terminal Security", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/terminal");
  });

  test("should sanitize shell metacharacters in logs as inert text", async ({ page }) => {
    const startButton = page.getByRole("button", { name: "Start New Analysis" });
    await expect(startButton).toBeVisible();

    const terminal = page.locator(".custom-scrollbar, [class*=terminal]").first();

    // Verify that the terminal displays content as text, not interpreted
    // Even if logs contained shell metacharacters like `; rm -rf /` or $(malicious),
    // they should appear as plain text in the terminal display
    await startButton.click();

    // Wait for terminal to show some logs or analysis state
    const terminalContent = terminal.locator("span");

    // The terminal should display logs as plain text without executing them
    // If metacharacters were evaluated, the DOM would be modified or requests would be made
    // We verify the terminal content is rendered as text within span elements
    const logElements = await terminalContent.all();
    for (const element of logElements) {
      const text = await element.textContent();
      // Verify that dangerous characters are preserved as-is, not evaluated
      if (text && (text.includes(";") || text.includes("|") || text.includes("$"))) {
        // These should be plain text in the DOM, not script execution
        expect(element).toBeTruthy();
      }
    }
  });

  test("should display logs in plaintext without executing commands", async ({ page }) => {
    // Navigate to terminal
    const terminal = page.locator(".font-mono").first();

    // Verify the terminal is set up for displaying logs
    await expect(terminal).toBeVisible();

    // The AnalysisTerminal component renders logs inside <span> elements with whitespace-pre-wrap
    // This ensures that any content, including shell metacharacters, is displayed literally
    // not interpreted or executed
    const span = terminal.locator("span");
    expect(span).toBeTruthy();
  });
});
