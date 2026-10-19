import { test, expect } from "@playwright/test";
import path from "path";

/**
 * E2E regression: uploading a known JSON report must render at least one
 * SVG node in the call-graph viewer (issue #858).
 */
test("call graph renders nodes for a known report", async ({ page }) => {
  await page.goto("/dashboard");

  // Upload a minimal fixture report that contains an auth_gap
  const fixtureReport = JSON.stringify({
    contract_name: "FixtureContract",
    file_path: "src/lib.rs",
    findings: [],
    auth_gaps: ["src/lib.rs:transfer"],
    panic_risks: [],
    storage_patterns: [],
    call_graph: [],
  });

  // Use the file input to upload the fixture
  const fileInput = page.locator("input[type=file]").first();
  await fileInput.setInputFiles({
    name: "fixture.json",
    mimeType: "application/json",
    buffer: Buffer.from(fixtureReport),
  });

  // Wait for the call graph SVG to appear
  const svgNode = page.locator("svg circle, svg ellipse, svg rect").first();
  await expect(svgNode).toBeVisible({ timeout: 10_000 });
});
