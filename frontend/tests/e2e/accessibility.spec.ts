import { expect, test } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";

test.describe("Dashboard accessibility", () => {
  test("dashboard page has no detectable accessibility violations", async ({ page }) => {
    await page.goto("/dashboard");

    const accessibilityScanResults = await new AxeBuilder({ page })
      .withTags(["wcag2a", "wcag2aa", "wcag21aa"])
      .analyze();

    expect(accessibilityScanResults.violations).toEqual([]);
  });

  test("severity filter buttons are keyboard navigable", async ({ page }) => {
    await page.goto("/dashboard");

    const mockReport = {
      summary: { total_findings: 2, has_critical: true, has_high: false },
      findings: {
        auth_gaps: [
          { code: "AUTH_GAP", function: "test.rs:func" },
        ],
        panic_issues: [
          { code: "PANIC_USAGE", function_name: "func", issue_type: "panic!", location: "test.rs:10" },
        ],
        arithmetic_issues: [],
        unsafe_patterns: [],
        ledger_size_warnings: [],
        custom_rules: [],
      },
    };

    await page.evaluate((report) => {
      const textarea = document.querySelector("textarea");
      if (textarea) {
        textarea.value = JSON.stringify(report);
        textarea.dispatchEvent(new Event("input", { bubbles: true }));
      }
    }, mockReport);

    await page.getByRole("button", { name: "Parse JSON" }).click();

    await page.waitForSelector('[role="group"][aria-label="Filter by severity"]');

    await page.keyboard.press("Tab");
    await expect(page.getByRole("button", { name: "All" })).toBeFocused();
    await page.keyboard.press("Enter");
    await expect(page.getByRole("button", { name: "All" })).toHaveAttribute("aria-pressed", "true");

    await page.keyboard.press("Tab");
    await expect(page.getByRole("button", { name: "Critical" })).toBeFocused();
  });

  test("tab navigation follows ARIA pattern", async ({ page }) => {
    await page.goto("/dashboard");

    const mockReport = {
      summary: { total_findings: 1, has_critical: true, has_high: false },
      findings: {
        auth_gaps: [{ code: "AUTH_GAP", function: "test.rs:func" }],
        panic_issues: [],
        arithmetic_issues: [],
        unsafe_patterns: [],
        ledger_size_warnings: [],
        custom_rules: [],
      },
    };

    await page.evaluate((report) => {
      const textarea = document.querySelector("textarea");
      if (textarea) {
        textarea.value = JSON.stringify(report);
        textarea.dispatchEvent(new Event("input", { bubbles: true }));
      }
    }, mockReport);

    await page.getByRole("button", { name: "Parse JSON" }).click();

    await page.waitForSelector('[role="tablist"]');

    const findingsTab = page.getByRole("tab", { name: "Findings" });
    const callGraphTab = page.getByRole("tab", { name: "Call Graph" });

    await expect(findingsTab).toHaveAttribute("aria-selected", "true");
    await expect(callGraphTab).toHaveAttribute("aria-selected", "false");

    await callGraphTab.click();
    await expect(callGraphTab).toHaveAttribute("aria-selected", "true");
    await expect(findingsTab).toHaveAttribute("aria-selected", "false");

    await expect(page.getByRole("tabpanel", { name: "Call Graph" })).toBeVisible();
  });
});

test.describe("Component accessibility", () => {
  test("call graph has accessible title and description", async ({ page }) => {
    await page.goto("/dashboard");

    const mockReport = {
      summary: { total_findings: 1, has_critical: true, has_high: false },
      findings: {
        auth_gaps: [{ code: "AUTH_GAP", function: "test.rs:func" }],
        panic_issues: [],
        arithmetic_issues: [],
        unsafe_patterns: [],
        ledger_size_warnings: [],
        custom_rules: [],
      },
    };

    await page.evaluate((report) => {
      const textarea = document.querySelector("textarea");
      if (textarea) {
        textarea.value = JSON.stringify(report);
        textarea.dispatchEvent(new Event("input", { bubbles: true }));
      }
    }, mockReport);

    await page.getByRole("button", { name: "Parse JSON" }).click();
    await page.getByRole("tab", { name: "Call Graph" }).click();

    const svg = page.locator('svg[role="img"][aria-label*="Contract call graph"]');
    await expect(svg).toBeVisible();

    await expect(svg.locator("title")).toContainText("Contract Call Graph");
    await expect(svg.locator("desc")).toContainText("function calls");
  });

  test("sanctity score chart has accessible label", async ({ page }) => {
    await page.goto("/dashboard");

    const mockReport = {
      summary: { total_findings: 1, has_critical: true, has_high: false },
      findings: {
        auth_gaps: [{ code: "AUTH_GAP", function: "test.rs:func" }],
        panic_issues: [],
        arithmetic_issues: [],
        unsafe_patterns: [],
        ledger_size_warnings: [],
        custom_rules: [],
      },
    };

    await page.evaluate((report) => {
      const textarea = document.querySelector("textarea");
      if (textarea) {
        textarea.value = JSON.stringify(report);
        textarea.dispatchEvent(new Event("input", { bubbles: true }));
      }
    }, mockReport);

    await page.getByRole("button", { name: "Parse JSON" }).click();

    const scoreSvg = page.locator('svg[aria-label*="Sanctity score"]');
    await expect(scoreSvg).toBeVisible();
  });

  test("severity bars have progress role", async ({ page }) => {
    await page.goto("/dashboard");

    const mockReport = {
      summary: { total_findings: 2, has_critical: true, has_high: true },
      findings: {
        auth_gaps: [{ code: "AUTH_GAP", function: "test.rs:func" }],
        panic_issues: [{ code: "PANIC_USAGE", function_name: "func", issue_type: "panic!", location: "test.rs:10" }],
        arithmetic_issues: [],
        unsafe_patterns: [],
        ledger_size_warnings: [],
        custom_rules: [],
      },
    };

    await page.evaluate((report) => {
      const textarea = document.querySelector("textarea");
      if (textarea) {
        textarea.value = JSON.stringify(report);
        textarea.dispatchEvent(new Event("input", { bubbles: true }));
      }
    }, mockReport);

    await page.getByRole("button", { name: "Parse JSON" }).click();

    const criticalBar = page.getByRole("progressbar", { name: /critical/i });
    await expect(criticalBar).toHaveAttribute("aria-valuenow", "1");

    const highBar = page.getByRole("progressbar", { name: /high/i });
    await expect(highBar).toHaveAttribute("aria-valuenow", "1");
  });
});
