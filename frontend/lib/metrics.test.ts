import { describe, it, expect } from "vitest";
import {
  Counter,
  Gauge,
  Histogram,
  metrics,
  renderMetrics,
  startAnalysisJob,
  trackAnalysisJob,
  withMetrics,
} from "./metrics";

describe("Prometheus exposition", () => {
  it("renders counters with sorted, escaped labels", () => {
    const c = new Counter("test_total", "A test counter.");
    c.inc({ route: "/x", method: "GET" });
    c.inc({ method: "GET", route: "/x" }, 2);
    c.inc({ route: 'a"b\\c\nd', method: "POST" });

    expect(c.render()).toBe(
      "# HELP test_total A test counter.\n" +
        "# TYPE test_total counter\n" +
        'test_total{method="GET",route="/x"} 3\n' +
        'test_total{method="POST",route="a\\"b\\\\c\\nd"} 1\n'
    );
  });

  it("rejects negative counter increments", () => {
    const c = new Counter("neg_total", "x");
    expect(() => c.inc({}, -1)).toThrow();
  });

  it("renders cumulative histogram buckets with +Inf, sum and count", () => {
    const h = new Histogram("lat_seconds", "Latency.", [0.1, 1]);
    h.observe({ route: "/a" }, 0.05);
    h.observe({ route: "/a" }, 0.5);
    h.observe({ route: "/a" }, 5);

    expect(h.render()).toBe(
      "# HELP lat_seconds Latency.\n" +
        "# TYPE lat_seconds histogram\n" +
        'lat_seconds_bucket{le="0.1",route="/a"} 1\n' +
        'lat_seconds_bucket{le="1",route="/a"} 2\n' +
        'lat_seconds_bucket{le="+Inf",route="/a"} 3\n' +
        'lat_seconds_sum{route="/a"} 5.55\n' +
        'lat_seconds_count{route="/a"} 3\n'
    );
  });

  it("renders gauges, including collected values", () => {
    const g = new Gauge("jobs", "Jobs.");
    g.inc({ route: "/a" });
    g.inc({ route: "/a" });
    g.dec({ route: "/a" });
    expect(g.render()).toContain('jobs{route="/a"} 1\n');

    const collected = new Gauge("mem_bytes", "Mem.", () => [{ labels: {}, value: 42 }]);
    expect(collected.render()).toContain("mem_bytes 42\n");
  });

  it("exposes every registry metric with HELP and TYPE lines", () => {
    const body = renderMetrics();
    for (const name of [
      "sanctifier_http_request_duration_seconds",
      "sanctifier_http_requests_total",
      "sanctifier_http_request_errors_total",
      "sanctifier_http_requests_in_flight",
      "sanctifier_analysis_jobs_in_flight",
      "sanctifier_analysis_jobs_total",
      "sanctifier_analysis_job_duration_seconds",
      "process_resident_memory_bytes",
    ]) {
      expect(body).toContain(`# TYPE ${name} `);
    }
  });
});

describe("withMetrics", () => {
  it("records status, latency and in-flight for successful responses", async () => {
    const route = "/test/ok";
    const handler = withMetrics(route, "GET", async () => {
      expect(metrics.httpRequestsInFlight.get({ route, method: "GET" })).toBe(1);
      return new Response("ok", { status: 201 });
    });

    const res = await handler();
    expect(res.status).toBe(201);

    const labels = { route, method: "GET", status: "201" };
    expect(metrics.httpRequestsTotal.get(labels)).toBe(1);
    expect(metrics.httpRequestDuration.getCount(labels)).toBe(1);
    expect(metrics.httpRequestErrorsTotal.get(labels)).toBe(0);
    expect(metrics.httpRequestsInFlight.get({ route, method: "GET" })).toBe(0);
  });

  it("counts 5xx responses as errors", async () => {
    const route = "/test/5xx";
    await withMetrics(route, "POST", async () => new Response(null, { status: 503 }))();

    expect(metrics.httpRequestErrorsTotal.get({ route, method: "POST", status: "503" })).toBe(1);
  });

  it("records thrown handlers as 500 and rethrows", async () => {
    const route = "/test/throw";
    const handler = withMetrics(route, "GET", async (): Promise<Response> => {
      throw new Error("boom");
    });

    await expect(handler()).rejects.toThrow("boom");
    const labels = { route, method: "GET", status: "500" };
    expect(metrics.httpRequestsTotal.get(labels)).toBe(1);
    expect(metrics.httpRequestErrorsTotal.get(labels)).toBe(1);
    expect(metrics.httpRequestsInFlight.get({ route, method: "GET" })).toBe(0);
  });

  it("passes handler arguments through", async () => {
    const handler = withMetrics("/test/args", "GET", async (a: number, b: string) =>
      Response.json({ a, b })
    );
    const res = await handler(1, "two");
    expect(await res.json()).toEqual({ a: 1, b: "two" });
  });
});

describe("analysis job tracking", () => {
  it("classifies exit codes and tracks in-flight jobs", async () => {
    const route = "/test/jobs";
    await trackAnalysisJob(route, async () => {
      expect(metrics.analysisJobsInFlight.get({ route })).toBe(1);
      return { exitCode: 0 };
    });
    // Exit 1 is the CLI's FINDINGS_FOUND code: a completed analysis.
    await trackAnalysisJob(route, async () => ({ exitCode: 1 }));
    await trackAnalysisJob(route, async () => ({ exitCode: 2 }));
    await trackAnalysisJob(route, async () => ({ exitCode: null }));

    expect(metrics.analysisJobsTotal.get({ route, outcome: "success" })).toBe(2);
    expect(metrics.analysisJobsTotal.get({ route, outcome: "failure" })).toBe(2);
    expect(metrics.analysisJobsInFlight.get({ route })).toBe(0);
  });

  it("classifies timeouts and other rejections", async () => {
    const route = "/test/job-errors";
    await expect(
      trackAnalysisJob(route, async () => {
        throw new Error("Analysis timed out after 30 seconds");
      })
    ).rejects.toThrow();
    await expect(
      trackAnalysisJob(route, async () => {
        throw new Error("spawn ENOENT");
      })
    ).rejects.toThrow();

    expect(metrics.analysisJobsTotal.get({ route, outcome: "timeout" })).toBe(1);
    expect(metrics.analysisJobsTotal.get({ route, outcome: "error" })).toBe(1);
  });

  it("ignores repeated finish calls", () => {
    const route = "/test/job-once";
    const finish = startAnalysisJob(route);
    finish("success");
    finish("error");

    expect(metrics.analysisJobsInFlight.get({ route })).toBe(0);
    expect(metrics.analysisJobsTotal.get({ route, outcome: "success" })).toBe(1);
    expect(metrics.analysisJobsTotal.get({ route, outcome: "error" })).toBe(0);
  });
});
