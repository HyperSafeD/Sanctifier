import { describe, it, expect, afterEach, vi } from "vitest";
import { GET } from "./route";

describe("GET /metrics", () => {
  afterEach(() => {
    vi.unstubAllEnvs();
  });

  it("serves Prometheus text format when no token is configured", async () => {
    vi.stubEnv("METRICS_TOKEN", "");
    const res = await GET(new Request("http://localhost/metrics"));

    expect(res.status).toBe(200);
    expect(res.headers.get("content-type")).toBe("text/plain; version=0.0.4; charset=utf-8");
    expect(await res.text()).toContain("# TYPE sanctifier_http_requests_total counter");
  });

  it("rejects scrapes without the configured bearer token", async () => {
    vi.stubEnv("METRICS_TOKEN", "s3cret");

    const missing = await GET(new Request("http://localhost/metrics"));
    expect(missing.status).toBe(401);

    const wrong = await GET(
      new Request("http://localhost/metrics", { headers: { authorization: "Bearer nope" } })
    );
    expect(wrong.status).toBe(401);

    const ok = await GET(
      new Request("http://localhost/metrics", { headers: { authorization: "Bearer s3cret" } })
    );
    expect(ok.status).toBe(200);
  });
});
