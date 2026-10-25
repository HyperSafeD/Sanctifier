# API Metrics (Prometheus + Grafana)

The hosted API (`frontend/app/api`) exposes Prometheus metrics for request
latency, error rate and analysis-job throughput. A Grafana dashboard built on
them is version-controlled in [`monitoring/grafana/sanctifier-api.json`](../monitoring/grafana/sanctifier-api.json).

## Scrape endpoint

`GET /metrics` returns the [Prometheus text exposition format](https://prometheus.io/docs/instrumenting/exposition_formats/)
(`Content-Type: text/plain; version=0.0.4`).

| Env var | Effect |
| --- | --- |
| `METRICS_TOKEN` | When set, scrapers must send `Authorization: Bearer <token>`; other requests get `401`. When unset the endpoint is open, so set it on any publicly reachable deployment. |

A ready-to-merge scrape job is in [`monitoring/prometheus/scrape-config.yml`](../monitoring/prometheus/scrape-config.yml).

Metrics are held in process memory. Each server instance reports its own
series, so scrape every instance (not a load-balanced URL) and aggregate in
PromQL. Counters reset when an instance restarts; `rate()` handles that.

## Metrics

| Metric | Type | Labels | Meaning |
| --- | --- | --- | --- |
| `sanctifier_http_request_duration_seconds` | histogram | `route`, `method`, `status` | Time until the route handler returns its response. For the streaming routes (`/api/analyze` GET, `/api/analyze/stream`) this is time to first byte. |
| `sanctifier_http_requests_total` | counter | `route`, `method`, `status` | All API requests. |
| `sanctifier_http_request_errors_total` | counter | `route`, `method`, `status` | Requests that returned 5xx or threw. Unhandled exceptions are recorded as `status="500"`. |
| `sanctifier_http_requests_in_flight` | gauge | `route`, `method` | Requests currently being handled. |
| `sanctifier_analysis_jobs_in_flight` | gauge | `route` | `sanctifier` CLI processes currently running. |
| `sanctifier_analysis_jobs_total` | counter | `route`, `outcome` | Finished CLI jobs. `outcome` is `success` (exit 0, or exit 1 = findings found), `failure` (any other exit or a signal), `timeout` (killed after 30 s) or `error` (spawn failed). |
| `sanctifier_analysis_job_duration_seconds` | histogram | `route`, `outcome` | Wall-clock time of each CLI job. |
| `process_resident_memory_bytes`, `nodejs_heap_size_used_bytes`, `process_start_time_seconds` | gauge | — | Process health. |

`route` is always the route template (e.g. `/api/contracts/[name]/source`),
never the concrete URL, so label cardinality stays bounded. Latency buckets
include the scan-latency SLO thresholds in [SLO.md](./SLO.md) (2, 3, 5, 8, 10, 12, 15 and 25 s).

### Instrumenting a new route

Wrap the handler instead of exporting it directly:

```ts
import { withMetrics, trackAnalysisJob } from "../../../lib/metrics";

async function handlePOST(request: NextRequest) {
  // Wrap CLI runs so they show up in the analysis-job series.
  const result = await trackAnalysisJob("/api/example", () => runAnalyzeCommand(path));
  // ...
}

export const POST = withMetrics("/api/example", "POST", handlePOST);
```

For CLI processes that are not a single promise (streaming), call
`startAnalysisJob(route)` and invoke the returned function from the child's
`close`/`error` handlers.

## Dashboard

Import `monitoring/grafana/sanctifier-api.json` in Grafana (**Dashboards → New
→ Import**), then pick your Prometheus data source in the **Data source**
dropdown at the top of the dashboard. The `Job`, `Instance` and `Route` variables filter every
panel.

| Row | Panels |
| --- | --- |
| Overview | Request rate, 5xx error ratio (thresholds at 0.5% / 1%), p95 latency (thresholds at the 8 s / 12 s SLO targets), analysis jobs in flight |
| Latency | p50/p95/p99 across selected routes; p50, p95 and p99 per route |
| Traffic & errors | 5xx ratio per endpoint, requests by endpoint and status, 4xx rate, requests in flight |
| Analysis jobs | Job throughput by outcome, job duration p50/p95/p99, jobs in flight, process memory |

The dashboard has been imported into Grafana 11.1 against Prometheus 2.53.
Every panel and variable query was checked to return data from the real
`lib/metrics.ts` exposition.

## Related

- [SLO.md](./SLO.md): targets these metrics measure
- [log-aggregation.md](./log-aggregation.md): structured logs (#1149), correlated by `request_id`
