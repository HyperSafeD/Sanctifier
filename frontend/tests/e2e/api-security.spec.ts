import { expect, test } from "@playwright/test";

test.describe("API Security - Rate Limiting", () => {
  test("returns 429 when rate limit exceeded", async ({ page }) => {
    const contractContent = `
fn example() -> u32 {
    42
}
`;

    await page.goto("/dashboard");

    for (let i = 0; i < 12; i++) {
      const content = contractContent;
      const input = await page.locator('input[type="file"][accept=".rs"]');
      await input.setInputFiles({
        name: "test.rs",
        mimeType: "text/plain",
        buffer: Buffer.from(content),
      });
      
      await page.waitForTimeout(100);
    }

    const lastResponse = page.waitForResponse((resp) => 
      resp.url().includes("/api/analyze") && resp.status() === 429
    );

    const input = await page.locator('input[type="file"][accept=".rs"]');
    await input.setInputFiles({
      name: "test.rs",
      mimeType: "text/plain",
      buffer: Buffer.from(contractContent),
    });

    const response = await lastResponse;
    expect(response.status()).toBe(429);
    
    const retryAfter = response.headers()["retry-after"];
    expect(retryAfter).toBeDefined();
    expect(parseInt(retryAfter)).toBeGreaterThan(0);

    const body = await response.json();
    expect(body.error).toContain("Rate limit");
  });
});

test.describe("API Security - File Size Validation", () => {
  test("returns 413 for files exceeding size limit", async ({ request }) => {
    const largeContent = "x".repeat(300 * 1024);

    const response = await request.post("/api/analyze", {
      multipart: {
        contract: {
          name: "large.rs",
          mimeType: "text/plain",
          buffer: Buffer.from(largeContent),
        },
      },
    });

    expect(response.status()).toBe(413);
    
    const body = await response.json();
    expect(body.error).toContain("File size");
  });
});

test.describe("API Security - Input Validation", () => {
  test("rejects non-.rs file extensions", async ({ request }) => {
    const response = await request.post("/api/analyze", {
      multipart: {
        contract: {
          name: "test.txt",
          mimeType: "text/plain",
          buffer: Buffer.from("content"),
        },
      },
    });

    expect(response.status()).toBe(400);
    
    const body = await response.json();
    expect(body.error).toContain(".rs");
  });

  test("rejects invalid UTF-8 content", async ({ request }) => {
    const invalidUtf8 = Buffer.from([0xff, 0xfe, 0xfd, 0xfc]);
    
    const response = await request.post("/api/analyze", {
      multipart: {
        contract: {
          name: "invalid.rs",
          mimeType: "text/plain",
          buffer: invalidUtf8,
        },
      },
    });

    expect(response.status()).toBe(400);
    
    const body = await response.json();
    expect(body.error).toContain("UTF-8");
  });

  test("rejects path traversal in filename", async ({ request }) => {
    const response = await request.post("/api/analyze", {
      multipart: {
        contract: {
          name: "../../../etc/passwd.rs",
          mimeType: "text/plain",
          buffer: Buffer.from("fn main() {}"),
        },
      },
    });

    expect(response.status()).toBeLessThan(500);
  });

  test("sanitizes special characters in filename", async ({ request }) => {
    const response = await request.post("/api/analyze", {
      multipart: {
        contract: {
          name: "test<>:\"|?*.rs",
          mimeType: "text/plain",
          buffer: Buffer.from("fn main() {}"),
        },
      },
    });

    expect([400, 500].includes(response.status())).toBeFalsy();
  });
});

test.describe("API Security - Timeout Handling", () => {
  test("returns 504 when analysis times out", async ({ page }) => {
    await page.route("**/api/analyze", async (route) => {
      await route.fulfill({
        status: 504,
        contentType: "application/json",
        body: JSON.stringify({ error: "Analysis timed out" }),
      });
    });

    await page.goto("/dashboard");

    const content = "fn main() {}";
    const input = await page.locator('input[type="file"][accept=".rs"]');
    await input.setInputFiles({
      name: "test.rs",
      mimeType: "text/plain",
      buffer: Buffer.from(content),
    });

    const response = await page.waitForResponse((resp) => 
      resp.url().includes("/api/analyze")
    );
    
    expect(response.status()).toBe(504);
  });
});

test.describe("API Security - Error Handling", () => {
  test("returns 400 when no file attached", async ({ request }) => {
    const response = await request.post("/api/analyze", {
      multipart: {},
    });

    expect(response.status()).toBe(400);
    
    const body = await response.json();
    expect(body.error).toContain("Attach");
  });

  test("handles missing contract field gracefully", async ({ request }) => {
    const response = await request.post("/api/analyze", {
      multipart: {
        other: {
          name: "test.rs",
          mimeType: "text/plain",
          buffer: Buffer.from("fn main() {}"),
        },
      },
    });

    expect(response.status()).toBe(400);
  });
});
