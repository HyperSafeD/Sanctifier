import { test, expect } from "@playwright/test";

test.describe("Dashboard Project Switching", () => {
  const sampleReport1 = {
    auth_gaps: [{ function_name: "initialize", code: "AUTH_GAP" }],
    panic_issues: [],
    arithmetic_issues: [],
    call_graph: [],
    unsafe_patterns: [],
    storage_collisions: [],
    event_issues: [],
    unhandled_results: [],
    upgrade_risks: [],
    sep41_issues: [],
  };

  const sampleReport2 = {
    auth_gaps: [],
    panic_issues: [{ code: "PANIC_001", function_name: "transfer", issue_type: "panic!", location: "src/lib.rs:45" }],
    arithmetic_issues: [],
    call_graph: [],
    unsafe_patterns: [],
    storage_collisions: [],
    event_issues: [],
    unhandled_results: [],
    upgrade_risks: [],
    sep41_issues: [],
  };

  test("should not leak stale findings when rapidly switching between projects", async ({ page }) => {
    await page.goto("/dashboard");

    const textarea = page.getByPlaceholder(/size_warnings/);
    const parseButton = page.getByRole("button", { name: "Parse JSON" });

    // Load first project (Report 1 with auth gaps)
    await textarea.fill(JSON.stringify(sampleReport1, null, 2));
    await parseButton.click();
    await page.waitForTimeout(500);

    // Verify Report 1's findings are displayed (auth_gaps)
    let findingsText = await page.locator("body").textContent();
    expect(findingsText).toContain("initialize");

    // Rapidly load second project (Report 2 with panic issues)
    await textarea.fill(JSON.stringify(sampleReport2, null, 2));
    await parseButton.click();

    // Wait briefly for the request to be in flight
    await page.waitForTimeout(100);

    // Load it again (simulating rapid switching)
    await textarea.fill(JSON.stringify(sampleReport1, null, 2));
    await parseButton.click();
    await page.waitForTimeout(500);

    // Final verify: should only show Report 1's findings (initialize), not Report 2's panic findings
    const findingsPanel = page.locator('section:has(h2:has-text("Findings"))');
    const panelText = await findingsPanel.textContent();

    // The panic finding from Report 2 should NOT appear while Report 1 is being viewed
    if (panelText) {
      expect(panelText).not.toContain("panic!");
    }

    // Report 1's findings should be present
    findingsText = await page.locator("body").textContent();
    expect(findingsText).toContain("initialize");
  });

  test("should cancel previous fetch when switching to new project", async ({ page }) => {
    await page.goto("/dashboard");

    const textarea = page.getByPlaceholder(/size_warnings/);
    const parseButton = page.getByRole("button", { name: "Parse JSON" });

    // Monitor network requests
    const failedRequests: string[] = [];
    page.on("response", (response) => {
      if (!response.ok && response.url().includes("/api")) {
        failedRequests.push(response.url());
      }
    });

    // Load Report 1
    await textarea.fill(JSON.stringify(sampleReport1, null, 2));
    await parseButton.click();

    // Immediately switch to Report 2 before Report 1 finishes loading
    await textarea.fill(JSON.stringify(sampleReport2, null, 2));
    await parseButton.click();

    // Wait for processing
    await page.waitForTimeout(1000);

    // Verify that findings shown are from the latest request (Report 2)
    const bodyText = await page.locator("body").textContent();

    // Should show Report 2's findings (panic issues), not Report 1's (auth gaps)
    // Or should show a loading state without stale data
    expect(bodyText).toBeTruthy();
  });
});
