# Frontend Dashboard Staged-Rollout (Canary) Strategy

This document outlines the staged-rollout and deployment strategy for shipping changes to the Sanctifier Next.js frontend dashboard (`frontend/app`) once serving production/mainnet users.

## 1. Overview & Objectives

Deploying changes directly to 100% of production traffic exposes all active users to potential regressions, RPC degradation, or frontend panics. To protect mainnet user experience and maintain system reliability, Sanctifier enforces a progressive, percentage-based staged rollout (canary deployment) pipeline coupled with automated rollback triggers.

### Architecture Features
- **Hosting Platform**: Next.js App Router deployed on Vercel / Edge Middleware or Kubernetes Ingress Controller.
- **Traffic Splitting**: Weighted routing at the edge layer to direct a percentage of traffic to Canary deployments without DNS re-propagation delays.
- **Skew Protection**: Version-pinned static assets (`/static/[buildId]/`) to prevent client-side chunk loading failures during active rollouts.
- **Feature Flags**: Dynamic runtime feature gating for high-risk UI changes and API endpoints.

---

## 2. Staged-Rollout Pipeline Stages

Every production release must progress sequentially through the following 6 stages:

```
[Stage 0: CI / E2E] ──> [Stage 1: Preview] ──> [Stage 2: 5% Canary]
                                                        │
[Stage 5: 100% GA]  <── [Stage 4: 50% Canary] <── [Stage 3: 25% Canary]
```

| Stage | Target Audience | Traffic Split | Soak Time | Approval Requirement |
|---|---|---|---|---|
| **Stage 0: CI/Automated** | Synthetic / E2E test suite | 0% production | N/A | Automated CI pass (Playwright + Vitest) |
| **Stage 1: Preview** | Internal QA & Security Team | Staging environment | 1 hour | Sign-off by Tech Lead |
| **Stage 2: Canary 5%** | Early adopters / Random traffic | 5% canary / 95% stable | 2 hours | Automated health check pass |
| **Stage 3: Canary 25%** | General users | 25% canary / 75% stable | 4 hours | Automated health check pass |
| **Stage 4: Canary 50%** | General users | 50% canary / 50% stable | 2 hours | Automated health check pass |
| **Stage 5: 100% GA** | All production traffic | 100% canary (promoted) | Permanent | Full Release Completed |

---

## 3. Telemetry & Log Aggregation Integration

Staged rollouts rely on real-time metric feeds from central log aggregation and performance monitoring (#1148, #1149, #1150):

- **Structured JSON Logging**: Standardized schema (`timestamp`, `level`, `request_id`, `component`) shipped to central aggregator (Grafana Loki / Datadog).
- **Correlation ID**: End-to-end `x-request-id` header propagated from Next.js API routes to the Rust CLI analysis engine.
- **Real-Time Dashboards**: Monitoring dashboards tracking 5xx error rate, p95 latency, RPC provider failover rates, and client error exceptions.

---

## 4. Concrete Rollback Triggers & Thresholds

A canary deployment must automatically halt and trigger an **immediate rollback** to the stable version if any of the following 4 threshold violations occur during any canary stage:

### Trigger 1: HTTP 5xx Error Rate Exceeded
- **Threshold**: HTTP 5xx responses exceed **1.0%** of total requests over a rolling 5-minute window, OR increase by **>2x** over the stable baseline.
- **Metric**: `sum(rate(http_requests_total{status=~"5.."}[5m])) / sum(rate(http_requests_total[5m])) * 100`

### Trigger 2: Latency Degradation
- **Threshold**: API route response latency 95th percentile (**p95**) exceeds **2,000ms**, OR degrades by **>50%** compared to the stable baseline.
- **Metric**: `histogram_quantile(0.95, sum(rate(http_request_duration_seconds_bucket[5m])) by (le))`

### Trigger 3: Client-Side Exception Spike
- **Threshold**: Unhandled JavaScript errors / React error boundary catches exceed **0.5%** of active sessions over a 10-minute window.
- **Metric**: `sum(rate(frontend_client_errors_total[10m])) / sum(rate(frontend_active_sessions_total[10m])) * 100`

### Trigger 4: RPC Provider Failover Exhaustion
- **Threshold**: RPC provider failover rate exceeds **0.1%** of analysis requests, or primary & secondary RPC endpoints experience simultaneous outages (#1146/#1155).
- **Metric**: `sum(rate(rpc_failover_exhaustion_total[5m]))`

---

## 5. Rollback Procedure & Execution

### Automated Rollback
When an automated monitoring alert fires for any Rollback Trigger during Canary (Stages 2–4):
1. Edge Routing middleware automatically resets traffic weight to **0% Canary / 100% Stable**.
2. Deployment status is flagged as `FAILED_ROLLBACK` in deployment logs.
3. On-call incident response team is alerted via Webhook / PagerDuty with the correlated `request_id` traces (#1157).

### Manual Emergency Rollback
To execute an immediate manual rollback via Vercel CLI or Edge Ingress:
```bash
# Instant rollback to previous stable production deployment
vercel rollback --yes

# Alternatively, set canary traffic weight to 0%
vercel promote <previous-deployment-id>
```

---

## 6. Verification Runbook

Before advancing between canary stages, perform the following verification checks:
1. Verify `x-request-id` propagation in central log query: `{component="frontend-api"} |= "request_id"`.
2. Inspect error rates in Grafana dashboard: zero critical 5xx errors recorded.
3. Check RPC failover logs for unexpected node fallback events.
