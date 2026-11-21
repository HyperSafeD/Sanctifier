import { describe, it, expect } from "vitest";
import { NextRequest } from "next/server";
import { GET, POST } from "./route";

const BASE = "http://localhost/api/recent-findings";

describe("/api/recent-findings GET - limit clamping", () => {
  it("clamps a client-supplied limit of 10000 to the server maximum", async () => {
    const findings = Array.from({ length: 150 }, (_, i) => ({ id: i }));
    await POST(
      new NextRequest(BASE, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ findings }),
      }),
    );

    const res = await GET(new NextRequest(`${BASE}?limit=10000`));
    const body = await res.json();

    expect(body.status).toBe("ok");
    expect(body.count).toBe(100);
    expect(body.findings).toHaveLength(100);
    expect(body.total_available).toBe(150);
  });
});
