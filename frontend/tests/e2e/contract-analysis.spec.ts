import path from "path";
import { expect, test } from "@playwright/test";

const mockAnalysisReport = {
  summary: {
    total_findings: 3,
    has_critical: true,
    has_high: true,
  },
  findings: {
    auth_gaps: [
      {
        code: "AUTH_GAP",
        function: "contracts/vulnerable-contract/src/lib.rs:transfer",
      },
    ],
    panic_issues: [
      {
        code: "PANIC_USAGE",
        function_name: "transfer",
        issue_type: "panic!",
        location: "contracts/vulnerable-contract/src/lib.rs:42",
      },
    ],
    arithmetic_issues: [
      {
        code: "ARITHMETIC_OVERFLOW",
        function_name: "mint",
        operation: "addition",
        suggestion: "Use checked_add before mutating balances.",
        location: "contracts/vulnerable-contract/src/lib.rs:57",
      },
    ],
    unsafe_patterns: [],
    ledger_size_warnings: [],
    custom_rules: [],
  },
};

test("uploads a contract and renders the returned analysis report", async ({ page }) => {
  await page.route("**/api/analyze", async (route) => {
    expect(route.request().method()).toBe("POST");

    const contentType = route.request().headers()["content-type"] ?? "";
    expect(contentType).toContain("multipart/form-data");

    const postData = route.request().postDataBuffer()?.toString("utf8") ?? "";
    expect(postData).toContain("vulnerable-contract.rs");

    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify(mockAnalysisReport),
    });
  });

  await page.goto("/dashboard");

  const contractPath = path.join(process.cwd(), "tests/e2e/fixtures/vulnerable-contract.rs");

  await page.getByTestId("contract-upload-input").setInputFiles(contractPath);

  await expect(
    page.getByRole("status").filter({ hasText: "Analysis report ready for vulnerable-contract.rs." })
  ).toBeVisible();
  await expect(page.getByText("Total: 3 findings")).toBeVisible();
  await expect(
    page.getByText("Modifying state without require_auth()")
  ).toBeVisible();
  await expect(page.getByText("Using panic!")).toBeVisible();
  await expect(page.getByText("Unchecked addition")).toBeVisible();
  await expect(page.locator("textarea")).toContainText('"panic_issues"');
});
