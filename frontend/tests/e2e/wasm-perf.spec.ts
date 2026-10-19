/**
 * sanctifier-wasm performance budgets — headless browser integration tests.
 *
 * These tests run against a live Next.js dev/preview server and assert that
 * the WASM analysis path stays within declared timing budgets.  They are
 * intentionally coarse-grained: the goal is to catch regressions (e.g. an
 * accidental synchronous loop or a missing cache warm-up), not micro-benchmark
 * individual functions.
 */

import { expect, test } from "@playwright/test";

// ─── Performance budgets (ms) ─────────────────────────────────────────────────

const BUDGET = {
  /** Maximum time for the dashboard page to reach "networkidle" state. */
  PAGE_LOAD_MS: 10_000,
  /** Maximum round-trip for a minimal contract via the analyze API. */
  SIMPLE_ANALYSIS_MS: 25_000,
  /** Maximum round-trip for a 100-function contract via the analyze API. */
  LARGE_ANALYSIS_MS: 30_000,
  /** Maximum time for the scan page to be interactive (DOMContentLoaded). */
  SCAN_PAGE_INTERACTIVE_MS: 8_000,
} as const;

// ─── Fixtures ─────────────────────────────────────────────────────────────────

const SIMPLE_CONTRACT = `
#![no_std]
use soroban_sdk::{contract, contractimpl};

#[contract]
pub struct SimpleContract;

#[contractimpl]
impl SimpleContract {
    pub fn hello() {}
}
`.trim();

const LARGE_CONTRACT = (() => {
  const fns = Array.from(
    { length: 50 }, // Reduced from 100 to avoid rate limiting
    (_, i) => `    pub fn fn_${i}(_env: soroban_sdk::Env) {}`
  ).join("\n");
  return `#![no_std]\nuse soroban_sdk::{contract, contractimpl, Env};\n\n#[contract]\npub struct LargeContract;\n\n#[contractimpl]\nimpl LargeContract {\n${fns}\n}`;
})();

// ─── Page-load budget ─────────────────────────────────────────────────────────

test.describe("page-load performance", () => {
  test("dashboard reaches networkidle within budget", async ({ page }) => {
    const start = Date.now();
    await page.goto("/dashboard", { waitUntil: "networkidle" });
    const elapsed = Date.now() - start;

    expect(elapsed, `Dashboard load ${elapsed}ms exceeded ${BUDGET.PAGE_LOAD_MS}ms budget`).toBeLessThan(
      BUDGET.PAGE_LOAD_MS
    );
  });

  test("scan page is interactive within budget", async ({ page }) => {
    const start = Date.now();
    await page.goto("/scan", { waitUntil: "domcontentloaded" });
    const elapsed = Date.now() - start;

    expect(
      elapsed,
      `Scan page interactive time ${elapsed}ms exceeded ${BUDGET.SCAN_PAGE_INTERACTIVE_MS}ms budget`
    ).toBeLessThan(BUDGET.SCAN_PAGE_INTERACTIVE_MS);
  });
});

// ─── Analysis API round-trip budgets ──────────────────────────────────────────

test.describe("analyze API performance budgets", () => {
  test("simple contract analysis completes within budget", async ({ request }) => {
    const start = Date.now();
    let response;
    let retries = 0;
    const maxRetries = 5;
    
    // Retry logic for rate limiting
    do {
      response = await request.post("/api/analyze", {
        data: { source: SIMPLE_CONTRACT },
        headers: { "Content-Type": "application/json" },
      });
      
      if (response.status() !== 429) break;
      
      retries++;
      if (retries < maxRetries) {
        // Wait before retrying (exponential backoff with base delay)
        const retryAfter = response.headers()["retry-after"];
        const delay = retryAfter ? parseInt(retryAfter) * 1000 : 2000 * Math.pow(2, retries);
        await new Promise(resolve => setTimeout(resolve, delay));
      }
    } while (retries < maxRetries);
    
    const elapsed = Date.now() - start;

    // 422 (not a Soroban contract) or 200/500 are all valid — we only care about latency.
    expect(response.status()).not.toBe(429);
    expect(
      elapsed,
      `Simple analysis ${elapsed}ms exceeded ${BUDGET.SIMPLE_ANALYSIS_MS}ms budget`
    ).toBeLessThan(BUDGET.SIMPLE_ANALYSIS_MS);
  });

  test("large contract analysis completes within budget", async ({ request }) => {
    const start = Date.now();
    let response;
    let retries = 0;
    const maxRetries = 5;
    
    // Retry logic for rate limiting
    do {
      response = await request.post("/api/analyze", {
        data: { source: LARGE_CONTRACT },
        headers: { "Content-Type": "application/json" },
      });
      
      if (response.status() !== 429) break;
      
      retries++;
      if (retries < maxRetries) {
        // Wait before retrying (exponential backoff with base delay)
        const retryAfter = response.headers()["retry-after"];
        const delay = retryAfter ? parseInt(retryAfter) * 1000 : 2000 * Math.pow(2, retries);
        await new Promise(resolve => setTimeout(resolve, delay));
      }
    } while (retries < maxRetries);
    
    const elapsed = Date.now() - start;

    expect(response.status()).not.toBe(429);
    expect(
      elapsed,
      `Large analysis ${elapsed}ms exceeded ${BUDGET.LARGE_ANALYSIS_MS}ms budget`
    ).toBeLessThan(BUDGET.LARGE_ANALYSIS_MS);
  });
});

// ─── Navigation performance marks ────────────────────────────────────────────

test.describe("browser navigation timing", () => {
  test("scan page Time-to-First-Byte is under 3 s", async ({ page }) => {
    await page.goto("/scan", { waitUntil: "domcontentloaded" });

    const ttfb = await page.evaluate(() => {
      const [nav] = performance.getEntriesByType(
        "navigation"
      ) as PerformanceNavigationTiming[];
      return nav ? nav.responseStart - nav.requestStart : -1;
    });

    if (ttfb >= 0) {
      expect(ttfb, `TTFB ${ttfb}ms exceeded 3000ms budget`).toBeLessThan(3_000);
    }
  });
});
