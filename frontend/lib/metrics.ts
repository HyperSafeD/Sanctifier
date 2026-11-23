/**
 * Prometheus metrics for Sanctifier Frontend API Routes
 *
 * A dependency-free registry that renders the Prometheus text exposition
 * format (https://prometheus.io/docs/instrumenting/exposition_formats/).
 * Scraped from GET /metrics (see app/metrics/route.ts).
 *
 * The registry lives on `globalThis` so every route bundle Next.js produces
 * shares one set of series within a server process.
 */

type Labels = Record<string, string>;

// Includes the scan-latency SLO thresholds from docs/SLO.md (2–25 s) so
// p50/p95/p99 can be compared against them without interpolation error.
const DEFAULT_LATENCY_BUCKETS = [
  0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1, 2, 3, 5, 8, 10, 12, 15, 25, 30,
];
const ANALYSIS_JOB_BUCKETS = [0.1, 0.25, 0.5, 1, 2.5, 5, 10, 20, 30, 60];

function labelKey(labels: Labels): string {
  return Object.keys(labels)
    .sort()
    .map((k) => `${k}=${labels[k]}`)
    .join(",");
}

function escapeLabelValue(value: string): string {
  return value.replace(/\\/g, "\\\\").replace(/\n/g, "\\n").replace(/"/g, '\\"');
}

function formatLabels(labels: Labels): string {
  const keys = Object.keys(labels).sort();
  if (keys.length === 0) return "";
  return `{${keys.map((k) => `${k}="${escapeLabelValue(labels[k])}"`).join(",")}}`;
}

function formatValue(value: number): string {
  if (value === Infinity) return "+Inf";
  if (value === -Infinity) return "-Inf";
  if (Number.isNaN(value)) return "NaN";
  return String(value);
}

interface Metric {
  readonly name: string;
  render(): string;
}

function header(name: string, help: string, type: string): string {
  return `# HELP ${name} ${help.replace(/\\/g, "\\\\").replace(/\n/g, "\\n")}\n# TYPE ${name} ${type}\n`;
}

export class Counter implements Metric {
  readonly name: string;
  private help: string;
  private series = new Map<string, { labels: Labels; value: number }>();

  constructor(name: string, help: string) {
    this.name = name;
    this.help = help;
  }

  inc(labels: Labels = {}, amount = 1): void {
    if (amount < 0) throw new Error(`counter ${this.name} cannot decrease`);
    const key = labelKey(labels);
    const entry = this.series.get(key);
    if (entry) entry.value += amount;
    else this.series.set(key, { labels: { ...labels }, value: amount });
  }

  get(labels: Labels = {}): number {
    return this.series.get(labelKey(labels))?.value ?? 0;
  }

  render(): string {
    let out = header(this.name, this.help, "counter");
    for (const { labels, value } of this.series.values()) {
      out += `${this.name}${formatLabels(labels)} ${formatValue(value)}\n`;
    }
    return out;
  }
}

export class Gauge implements Metric {
  readonly name: string;
  private help: string;
  private collect?: () => Array<{ labels: Labels; value: number }>;
  private series = new Map<string, { labels: Labels; value: number }>();

  constructor(
    name: string,
    help: string,
    collect?: () => Array<{ labels: Labels; value: number }>
  ) {
    this.name = name;
    this.help = help;
    this.collect = collect;
  }

  set(labels: Labels, value: number): void {
    this.series.set(labelKey(labels), { labels: { ...labels }, value });
  }

  inc(labels: Labels = {}, amount = 1): void {
    this.set(labels, this.get(labels) + amount);
  }

  dec(labels: Labels = {}, amount = 1): void {
    this.set(labels, this.get(labels) - amount);
  }

  get(labels: Labels = {}): number {
    return this.series.get(labelKey(labels))?.value ?? 0;
  }

  render(): string {
    if (this.collect) {
      for (const { labels, value } of this.collect()) this.set(labels, value);
    }
    let out = header(this.name, this.help, "gauge");
    for (const { labels, value } of this.series.values()) {
      out += `${this.name}${formatLabels(labels)} ${formatValue(value)}\n`;
    }
    return out;
  }
}

type HistogramSeries = {
  labels: Labels;
  buckets: number[];
  sum: number;
  count: number;
};

export class Histogram implements Metric {
  readonly name: string;
  private help: string;
  private series = new Map<string, HistogramSeries>();
  private readonly bounds: number[];

  constructor(name: string, help: string, buckets: number[]) {
    this.name = name;
    this.help = help;
    this.bounds = [...buckets].sort((a, b) => a - b);
  }

  observe(labels: Labels, value: number): void {
    const key = labelKey(labels);
    let entry = this.series.get(key);
    if (!entry) {
      entry = {
        labels: { ...labels },
        buckets: new Array(this.bounds.length).fill(0),
        sum: 0,
        count: 0,
      };
      this.series.set(key, entry);
    }
    for (let i = 0; i < this.bounds.length; i++) {
      if (value <= this.bounds[i]) entry.buckets[i]++;
    }
    entry.sum += value;
    entry.count++;
  }

  getCount(labels: Labels = {}): number {
    return this.series.get(labelKey(labels))?.count ?? 0;
  }

  render(): string {
    let out = header(this.name, this.help, "histogram");
    for (const { labels, buckets, sum, count } of this.series.values()) {
      this.bounds.forEach((bound, i) => {
        out += `${this.name}_bucket${formatLabels({ ...labels, le: formatValue(bound) })} ${buckets[i]}\n`;
      });
      out += `${this.name}_bucket${formatLabels({ ...labels, le: "+Inf" })} ${count}\n`;
      out += `${this.name}_sum${formatLabels(labels)} ${formatValue(sum)}\n`;
      out += `${this.name}_count${formatLabels(labels)} ${count}\n`;
    }
    return out;
  }
}

function createRegistry() {
  const startTimeSeconds = Math.floor(Date.now() / 1000);

  const httpRequestDuration = new Histogram(
    "sanctifier_http_request_duration_seconds",
    "API request latency in seconds, measured until the response is returned.",
    DEFAULT_LATENCY_BUCKETS
  );
  const httpRequestsTotal = new Counter(
    "sanctifier_http_requests_total",
    "Total API requests by route, method and status code."
  );
  const httpRequestErrorsTotal = new Counter(
    "sanctifier_http_request_errors_total",
    "API requests that ended in a 5xx response or an unhandled exception."
  );
  const httpRequestsInFlight = new Gauge(
    "sanctifier_http_requests_in_flight",
    "API requests currently being handled."
  );
  const analysisJobsInFlight = new Gauge(
    "sanctifier_analysis_jobs_in_flight",
    "sanctifier CLI analysis processes currently running."
  );
  const analysisJobsTotal = new Counter(
    "sanctifier_analysis_jobs_total",
    "Completed sanctifier CLI analysis jobs by outcome (success | failure | timeout | error)."
  );
  const analysisJobDuration = new Histogram(
    "sanctifier_analysis_job_duration_seconds",
    "Wall-clock duration of sanctifier CLI analysis jobs in seconds.",
    ANALYSIS_JOB_BUCKETS
  );
  const processStartTime = new Gauge(
    "process_start_time_seconds",
    "Start time of the process since unix epoch in seconds.",
    () => [{ labels: {}, value: startTimeSeconds }]
  );
  const processResidentMemory = new Gauge(
    "process_resident_memory_bytes",
    "Resident memory size in bytes.",
    () => [{ labels: {}, value: process.memoryUsage().rss }]
  );
  const nodejsHeapUsed = new Gauge(
    "nodejs_heap_size_used_bytes",
    "V8 heap used in bytes.",
    () => [{ labels: {}, value: process.memoryUsage().heapUsed }]
  );

  const all: Metric[] = [
    httpRequestDuration,
    httpRequestsTotal,
    httpRequestErrorsTotal,
    httpRequestsInFlight,
    analysisJobsInFlight,
    analysisJobsTotal,
    analysisJobDuration,
    processStartTime,
    processResidentMemory,
    nodejsHeapUsed,
  ];

  return {
    httpRequestDuration,
    httpRequestsTotal,
    httpRequestErrorsTotal,
    httpRequestsInFlight,
    analysisJobsInFlight,
    analysisJobsTotal,
    analysisJobDuration,
    render: () => all.map((m) => m.render()).join(""),
  };
}

type Registry = ReturnType<typeof createRegistry>;

const REGISTRY_KEY = Symbol.for("sanctifier.metrics.registry");
const globalStore = globalThis as unknown as { [REGISTRY_KEY]?: Registry };

export const metrics: Registry = (globalStore[REGISTRY_KEY] ??= createRegistry());

export const METRICS_CONTENT_TYPE = "text/plain; version=0.0.4; charset=utf-8";

export function renderMetrics(): string {
  return metrics.render();
}

/**
 * Wrap a route handler so its latency, status and errors are recorded.
 *
 * `route` must be the route template (e.g. "/api/contracts/[name]/source"),
 * never the concrete URL, to keep label cardinality bounded.
 */
export function withMetrics<Args extends unknown[], R extends Response>(
  route: string,
  method: string,
  handler: (...args: Args) => Promise<R> | R
): (...args: Args) => Promise<R> {
  return async (...args: Args) => {
    const start = performance.now();
    metrics.httpRequestsInFlight.inc({ route, method });
    let status = "500";
    try {
      const response = await handler(...args);
      status = String(response.status);
      return response;
    } finally {
      const labels = { route, method, status };
      metrics.httpRequestsInFlight.dec({ route, method });
      metrics.httpRequestsTotal.inc(labels);
      metrics.httpRequestDuration.observe(labels, (performance.now() - start) / 1000);
      if (status.startsWith("5")) metrics.httpRequestErrorsTotal.inc(labels);
    }
  };
}

export type AnalysisOutcome = "success" | "failure" | "timeout" | "error";

/**
 * Map a sanctifier CLI exit code to a job outcome. The CLI exits 1
 * (FINDINGS_FOUND) when it reports issues, which is a completed analysis;
 * 2 (ERROR), signals (null) and anything else are failures.
 */
export function outcomeForExitCode(exitCode: number | null): AnalysisOutcome {
  return exitCode === 0 || exitCode === 1 ? "success" : "failure";
}

/**
 * Track one sanctifier CLI analysis job. Call the returned function exactly
 * once when the job ends; later calls are ignored.
 */
export function startAnalysisJob(route: string): (outcome: AnalysisOutcome) => void {
  const start = performance.now();
  metrics.analysisJobsInFlight.inc({ route });
  let finished = false;
  return (outcome) => {
    if (finished) return;
    finished = true;
    metrics.analysisJobsInFlight.dec({ route });
    metrics.analysisJobsTotal.inc({ route, outcome });
    metrics.analysisJobDuration.observe({ route, outcome }, (performance.now() - start) / 1000);
  };
}

/** Run a promise-returning CLI job and record it via {@link startAnalysisJob}. */
export async function trackAnalysisJob<T extends { exitCode: number | null }>(
  route: string,
  job: () => Promise<T>
): Promise<T> {
  const finish = startAnalysisJob(route);
  try {
    const result = await job();
    finish(outcomeForExitCode(result.exitCode));
    return result;
  } catch (error) {
    finish(error instanceof Error && error.message.includes("timed out") ? "timeout" : "error");
    throw error;
  }
}
