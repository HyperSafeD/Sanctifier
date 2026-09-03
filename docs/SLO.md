# Service Level Objectives (SLO)

**Last Updated**: July 29, 2026  
**Version**: 1.0.0  
**Applies To**: Sanctifier Hosted API & Dashboard (mainnet-stable)

## Overview

This document defines Service Level Objectives (SLOs) for the Sanctifier hosted scanning service. These targets guide operational decisions, capacity planning, and incident response priorities.

**Target Audience**: Users, operators, and maintainers of the Sanctifier hosted service.

---

## What Are SLOs?

**Service Level Objectives (SLOs)** are internal targets that define the expected reliability and performance of a service. They are:

- **Measurable** - Based on quantitative metrics
- **Achievable** - Realistic given current infrastructure
- **User-Centric** - Reflect what users care about (latency, availability, throughput)

**Service Level Agreements (SLAs)** are contractual commitments with consequences for non-compliance. This document defines SLOs only. Legal SLA terms for paid tiers are covered separately in the Terms of Service.

---

## Core SLO Targets

### 1. API Availability

**Definition**: Percentage of time the API endpoint (`/api/v1/analyze`) returns successful responses (HTTP 2xx or 4xx, excluding 5xx errors).

| Metric | Target | Measurement Window | Measurement Method |
|--------|--------|-------------------|-------------------|
| **Uptime** | ≥ 99.5% | Monthly (30 days) | Prometheus `up{job="api"}` |
| **Success Rate** | ≥ 99.0% | Monthly | `(2xx + 4xx) / total_requests` |

**What Counts as Downtime**:
- ✅ HTTP 500, 502, 503, 504 (server errors)
- ✅ Connection timeouts or refused connections
- ✅ DNS resolution failures

**What Does NOT Count as Downtime**:
- ❌ HTTP 400, 401, 403, 413, 422, 429 (client errors)
- ❌ Planned maintenance windows (announced ≥24h in advance)
- ❌ Upstream provider outages (Stellar RPC, GitHub, etc.)

**Target**: **99.5% uptime** = max **3.6 hours downtime per month**

---

### 2. Scan Latency

**Definition**: Time from receiving an analysis request to returning the complete result (p50, p95, p99 percentiles).

| Percentile | Free Tier Target | Pro Tier Target | Enterprise Target | Measurement Window |
|------------|------------------|-----------------|-------------------|-------------------|
| **p50** (median) | ≤ 5 seconds | ≤ 3 seconds | ≤ 2 seconds | 5 minutes |
| **p95** | ≤ 12 seconds | ≤ 8 seconds | ≤ 5 seconds | 5 minutes |
| **p99** | ≤ 25 seconds | ≤ 15 seconds | ≤ 10 seconds | 5 minutes |

**Measurement Method**:
- Start: Request received by API gateway
- End: Complete JSON/SARIF response sent to client
- Metric: Prometheus histogram `http_request_duration_seconds{endpoint="/api/v1/analyze"}`

**Exclusions**:
- Requests that hit rate limits (HTTP 429) - not measured
- Requests exceeding file size limits (HTTP 413) - not measured
- Timeouts due to malformed contracts (HTTP 422) - measured separately

**Notes**:
- Large contracts (>500 KB) may exceed p99 targets
- Complex analysis rules (e.g., symbolic execution) may increase latency
- Cold-start latency (first request after idle period) measured separately

---

### 3. Queue Wait Time

**Definition**: Time a request spends waiting in the job queue before analysis begins (when concurrent limit is reached).

| Tier | Max Wait Time | Measurement Method |
|------|---------------|-------------------|
| **Free** | ≤ 30 seconds | `queue_wait_seconds{tier="free"}` p95 |
| **Pro** | ≤ 10 seconds | `queue_wait_seconds{tier="pro"}` p95 |
| **Enterprise** | ≤ 2 seconds | `queue_wait_seconds{tier="enterprise"}` p95 |

**Target**: **p95 queue wait time** within tier-specific limits

**What Happens When Exceeded**:
- Requests waiting >60 seconds are auto-rejected with HTTP 503
- `retry-after` header suggests when to retry
- Users receive a "Service Busy" error

---

### 4. Dashboard Page Load Time

**Definition**: Time for the Sanctifier web dashboard to become interactive (Largest Contentful Paint - LCP).

| Page | Target (p75) | Measurement Method |
|------|-------------|-------------------|
| **Homepage** | ≤ 2.5 seconds | Vercel Analytics LCP |
| **Scan Results** | ≤ 3.0 seconds | Vercel Analytics LCP |
| **Contract Upload** | ≤ 2.0 seconds | Vercel Analytics LCP |

**Measurement Window**: Rolling 7 days  
**Measurement Tool**: Vercel Web Vitals / Lighthouse CI

**Target**: **p75 LCP ≤ 2.5 seconds** across all pages

---

### 5. Error Budget

**Definition**: Allowed failure rate before triggering incident response.

| Service Component | Monthly Error Budget | Trigger Action |
|-------------------|---------------------|----------------|
| **API** | 0.5% (99.5% success rate) | Page on-call engineer if exceeded |
| **Dashboard** | 1.0% (99.0% page load success) | Alert Slack #alerts channel |
| **CLI Auto-Update** | 2.0% (98.0% download success) | Log for post-mortem review |

**Error Budget Calculation**:
```
Error Budget = (1 - SLO) × Total Requests
Example: 99.5% SLO over 1M requests = 5,000 allowed errors
```

**When Error Budget is Exhausted**:
1. Stop non-critical releases and feature work
2. Focus on reliability improvements
3. Conduct post-mortem to identify root cause
4. Implement preventive measures

---

## Measurement & Monitoring

### Data Sources

| Metric | Tool | Dashboard | Alert Channel |
|--------|------|-----------|---------------|
| **API Uptime** | Prometheus + Grafana | [API Health Dashboard](#) | #alerts (Slack) |
| **Scan Latency** | Prometheus histograms | [Latency Dashboard](#) | #performance |
| **Queue Depth** | Redis metrics | [Queue Monitor](#) | #capacity |
| **Dashboard Performance** | Vercel Analytics | [Web Vitals](#) | #frontend |
| **Error Rate** | Sentry + Prometheus | [Error Budget](#) | #incidents |

### Alerting Thresholds

| Condition | Severity | Response Time | Escalation |
|-----------|----------|---------------|------------|
| API uptime <99.5% over 1 hour | **Critical** | Immediate page | On-call engineer |
| p95 latency >15s for 5 minutes | **High** | 15 minutes | #alerts channel |
| Queue wait p95 >60s for 10 min | **Medium** | 30 minutes | Capacity planning team |
| Error budget 50% consumed | **Low** | Next business day | Weekly review |

---

## SLO Violation Response

### Incident Severity Classification

| Severity | Definition | Example | Target Response |
|----------|-----------|---------|-----------------|
| **SEV-1 (Critical)** | Complete service outage | API returns 503 for all requests | <15 minutes |
| **SEV-2 (High)** | Partial outage or severe degradation | p95 latency >3x SLO | <1 hour |
| **SEV-3 (Medium)** | Minor degradation, still functional | p95 latency 1.5-3x SLO | <4 hours |
| **SEV-4 (Low)** | No user impact, monitoring alert | Error budget 75% consumed | Next business day |

### Incident Response Workflow

1. **Detection**: Automated alert fires in #alerts
2. **Acknowledgment**: On-call engineer acknowledges within SLA
3. **Investigation**: Identify root cause using logs/metrics
4. **Mitigation**: Implement fix or rollback
5. **Communication**: Update status page with user-facing message
6. **Resolution**: Confirm SLO metrics return to normal
7. **Post-Mortem**: Document incident, root cause, and preventive actions (within 48 hours)

---

## SLO Review & Updates

### Review Cadence

- **Monthly**: Review SLO performance against targets
- **Quarterly**: Assess if SLOs need adjustment based on:
  - Actual performance data
  - User feedback and complaints
  - Infrastructure changes
  - Cost vs. reliability tradeoffs

### Revision Process

1. Propose SLO change with justification (GitHub issue)
2. Review with engineering and product teams
3. Communicate changes to users ≥30 days before effective date
4. Update this document with new version number

**Version History**:
- **v1.0.0** (2026-07-29): Initial SLO targets for mainnet-stable

---

## Out of Scope

This SLO document does **NOT** cover:

- ❌ **Legal SLA terms** - See Terms of Service for contractual commitments
- ❌ **CLI local analysis** - Performance depends on user's hardware
- ❌ **VS Code extension** - Responsiveness depends on VS Code host
- ❌ **Third-party integrations** - GitHub Actions, Discord bots (separate SLOs if needed)
- ❌ **Mainnet contract deployment** - Stellar network uptime outside our control

---

## Baseline Data & Assumptions

**Note**: This initial SLO document is based on **conservative estimates** and industry benchmarks, as baseline metrics from production monitoring (#1150) are not yet available.

### Assumptions

- **Traffic estimate**: 10,000 API requests/day at mainnet launch
- **Average request size**: 150 KB
- **Analysis workload**: 80% simple contracts (<5s), 15% medium (5-15s), 5% complex (15-30s)
- **Infrastructure**: Vercel serverless + Redis queue + PostgreSQL
- **Geographic distribution**: 70% US, 20% Europe, 10% Asia-Pacific

### Expected Adjustments

Once Grafana/Prometheus dashboards (#1150) provide actual performance data, we will:

1. **Tighten or relax targets** based on observed p95/p99 latency
2. **Add tier-specific SLOs** for Free vs. Pro if variance is significant
3. **Set queue depth alerts** based on actual concurrency patterns
4. **Refine error budgets** based on real failure modes

**Target Date for First Revision**: 30 days after mainnet launch (August 29, 2026)

---

## Reporting & Transparency

### Public Status Page

Real-time service status available at: **[status.sanctifier.hypersafeD.io](#)** (see #1147)

Displays:
- ✅ Current operational status (Operational / Degraded / Outage)
- 📊 90-day uptime history
- 🔔 Active and resolved incidents
- 📅 Scheduled maintenance windows

### Historical Performance Reports

Monthly SLO performance reports published at: **[sanctifier.hypersafeD.io/status/reports](#)**

Includes:
- Achieved uptime % vs. 99.5% target
- Latency percentiles (p50/p95/p99) vs. targets
- Error budget consumption
- Incident summaries and post-mortems

---

## Contact & Support

### For Users

- **Status Page**: [status.sanctifier.hypersafeD.io](#)
- **Support Email**: [support@hypersafeD.io](mailto:support@hypersafeD.io)
- **GitHub Issues**: [HyperSafeD/Sanctifier/issues](https://github.com/HyperSafeD/Sanctifier/issues)

### For Operators

- **On-Call Rotation**: PagerDuty schedule
- **Runbooks**: [docs/runbooks/](./runbooks/)
- **Incident Channel**: #incidents (Slack)

---

## Related Documentation

- [API Pricing & Limits](./api-pricing-limits.md) - Rate limits and tier quotas
- [Architecture Overview](../ARCHITECTURE.md) - System design and dependencies
- [Monitoring Setup](#) - Prometheus/Grafana configuration (#1150)
- [Incident Response Playbook](#) - Detailed incident handling procedures
- [Status Page](#) - Public-facing service status (#1147)

---

## Glossary

| Term | Definition |
|------|------------|
| **SLO** | Service Level Objective - Internal reliability target |
| **SLA** | Service Level Agreement - Contractual commitment with penalties |
| **SLI** | Service Level Indicator - Measurable metric (latency, error rate) |
| **Error Budget** | Allowed failure rate before triggering corrective action |
| **p50/p95/p99** | Percentile latency (50th, 95th, 99th percentile of requests) |
| **LCP** | Largest Contentful Paint - Web performance metric |
| **Uptime** | Percentage of time service is available and functional |

---

**Document Owner**: Platform Team  
**Last Review**: 2026-07-29  
**Next Review**: 2026-08-29 (30 days post-mainnet launch)

---

## Appendix: SLO Calculation Examples

### Example 1: Monthly Uptime

```
Target: 99.5% uptime per month
Month: 30 days = 43,200 minutes

Allowed downtime = (1 - 0.995) × 43,200 = 216 minutes = 3.6 hours

If actual downtime = 180 minutes (3 hours):
  Achieved uptime = (43,200 - 180) / 43,200 = 99.58% ✅ PASS
```

### Example 2: Error Budget

```
Target: 99.0% success rate
Monthly requests: 1,000,000

Error budget = 1,000,000 × (1 - 0.990) = 10,000 allowed errors

Week 1: 2,000 errors (20% budget consumed)
Week 2: 3,500 errors (35% consumed, 55% total)
Week 3: 5,000 errors (50% consumed, 105% total) ❌ EXCEEDED

Action: Halt feature releases, focus on reliability
```

### Example 3: Latency SLO Compliance

```
Target: p95 latency ≤ 12 seconds (Free tier)
Sample: 1,000 requests over 5 minutes

Latencies sorted:
  p50 = 4.2 seconds ✅
  p95 = 11.8 seconds ✅
  p99 = 24.3 seconds (no p99 SLO for Free tier)

Result: PASS (p95 within target)
```

---

*This document is a living document and will be updated as the service evolves and more data becomes available.*
