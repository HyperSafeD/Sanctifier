# Unified Structured Logging & Log Aggregation Architecture

This document describes the structured logging, correlation context, and central log aggregation architecture for Sanctifier across the Rust CLI, Analysis Core, and Next.js Frontend API routes.

## 1. Unified JSON Log Schema

All Sanctifier components emit structured JSON logs conforming to the unified schema:

```json
{
  "timestamp": "2026-08-31T04:15:30.123Z",
  "level": "info",
  "request_id": "req-9a8b7c6d-5e4f",
  "component": "frontend-api",
  "message": "Received contract analysis request",
  "path": "/api/analyze",
  "method": "POST",
  "client_ip": "127.0.0.1",
  "duration_ms": 142
}
```

### Required Fields
- `timestamp`: ISO 8601 string in UTC.
- `level`: Log level (`info`, `warn`, `error`, `debug`).
- `request_id`: Correlation identifier (`x-request-id` header or `SANCTIFIER_REQUEST_ID` environment variable).
- `component`: Source component name (`frontend-api`, `sanctifier-cli`, `sanctifier-core`).
- `message`: Human-readable summary of the log event.

---

## 2. Distributed Context & Trace Correlation (`request_id`)

The `request_id` (or `x-request-id`) is propagated across all system boundaries to enable end-to-end incident investigation:

```
[ HTTP Client ] ──(x-request-id)──> [ Next.js API Route ]
                                            │
                                  (SANCTIFIER_REQUEST_ID)
                                            ▼
[ Central Loki Log DB ] <──(JSON)── [ Sanctifier CLI / Core Engine ]
```

1. **Ingress**: Next.js API routes (`/api/analyze`, `/api/ai/explain`) inspect the incoming `x-request-id` HTTP header. If absent, a unique UUID (`req-[hash]`) is generated.
2. **Egress Headers**: API routes return `x-request-id` in HTTP response headers.
3. **Subprocess Propagation**: Next.js spawns `sanctifier analyze` with `--request-id <req_id>` and `SANCTIFIER_REQUEST_ID=<req_id>` in the environment.
4. **Rust Telemetry**: The CLI tracing subscriber formats all log spans with `request_id=<req_id>` and emits them via JSON on `stderr`.

---

## 3. Log Aggregation & Shipping Setup

### Shipping Options
1. **Stdout/Stderr Aggregation (Recommended for Kubernetes/Vercel/Docker)**:
   - Components stream structured JSON lines directly to `stdout`/`stderr`.
   - Log shippers (Promtail, Vector, Datadog Agent, AWS CloudWatch Agent) collect streams automatically.

2. **Direct HTTP Ingestion (Grafana Loki)**:
   - Configure `LOG_AGGREGATOR_URL` / `LOKI_URL` to ship JSON batches directly over HTTP POST.

### Promtail / Grafana Loki Sample Configuration
```yaml
scrape_configs:
  - job_name: sanctifier-logs
    static_configs:
      - targets:
          - localhost
        labels:
          job: sanctifier
          __path__: /var/log/sanctifier/*.log
    pipeline_stages:
      - json:
          expressions:
            timestamp: timestamp
            level: level
            request_id: request_id
            component: component
            message: message
      - labels:
          level:
          component:
          request_id:
      - timestamp:
          source: timestamp
          format: RFC3339
```

---

## 4. Correlated Log Investigation Query Example (Grafana Loki)

To trace a specific request across frontend API and Rust analysis core in Grafana Loki:

```logql
{job="sanctifier"} | json | request_id = "req-9a8b7c6d-5e4f"
```
