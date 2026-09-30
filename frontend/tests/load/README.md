# Load Testing Suite

**Issue**: #1154  
**Purpose**: Validate SLO targets from `docs/SLO.md` under mainnet-scale traffic

## Overview

This directory contains k6-based load testing scripts that simulate realistic concurrent user traffic against the Sanctifier hosted API and dashboard.

### Test Scripts

| Script | Purpose | Traffic Pattern |
|--------|---------|-----------------|
| `api-scan-load-test.js` | Main API endpoint testing | Concurrent scan requests with realistic payload sizes |
| (Future) `dashboard-load-test.js` | Frontend page load testing | Simulates users browsing results |
| (Future) `mixed-workflow-test.js` | Combined API + dashboard | End-to-end user journeys |

---

## Prerequisites

### Install K6

**macOS (Homebrew)**:
```bash
brew install k6
```

**Linux (Debian/Ubuntu)**:
```bash
sudo gpg -k
sudo gpg --no-default-keyring --keyring /usr/share/keyrings/k6-archive-keyring.gpg --keyserver hkp://keyserver.ubuntu.com:80 --recv-keys C5AD17C747E3415A3642D57D77C6C491D6AC1D69
echo "deb [signed-by=/usr/share/keyrings/k6-archive-keyring.gpg] https://dl.k6.io/deb stable main" | sudo tee /etc/apt/sources.list.d/k6.list
sudo apt-get update
sudo apt-get install k6
```

**Windows (Chocolatey)**:
```powershell
choco install k6
```

**Docker**:
```bash
docker pull grafana/k6
```

### Environment Setup

Copy `.env.example` to `.env` and configure:

```bash
# .env
BASE_URL=https://staging.sanctifier.io
API_KEY=your-staging-api-key-here
```

---

## Running Tests

### Quick Start

```bash
cd frontend/tests/load

# Smoke test (1 VU, 1 minute)
k6 run --env SCENARIO=smoke api-scan-load-test.js

# Load test (normal mainnet traffic)
k6 run --env SCENARIO=load api-scan-load-test.js

# Stress test (3x normal load)
k6 run --env SCENARIO=stress api-scan-load-test.js

# Spike test (sudden traffic surge)
k6 run --env SCENARIO=spike api-scan-load-test.js

# Soak test (30 minutes sustained load)
k6 run --env SCENARIO=soak api-scan-load-test.js
```

### Custom Configuration

```bash
# Test against localhost
k6 run --env BASE_URL=http://localhost:3000 --env SCENARIO=smoke api-scan-load-test.js

# Override API key
k6 run --env API_KEY=sk-custom-key api-scan-load-test.js

# Save results to file
k6 run api-scan-load-test.js --out json=results/output.json

# Run with increased verbosity
k6 run --verbose api-scan-load-test.js
```

### Docker Usage

```bash
docker run --rm -v $(pwd):/scripts grafana/k6 run /scripts/api-scan-load-test.js
```

---

## Test Scenarios

### 1. Smoke Test
- **Purpose**: Verify functionality, catch critical bugs
- **Load**: 1 VU for 1 minute
- **Use Case**: Quick sanity check before larger tests

### 2. Load Test
- **Purpose**: Validate SLOs under normal mainnet traffic
- **Load**: Ramps 0→10→20 VUs over 15 minutes
- **Traffic Model**: ~10K requests/day = 7 req/min sustained
- **Use Case**: Pre-launch SLO validation

### 3. Stress Test
- **Purpose**: Find breaking point, identify scalability limits
- **Load**: Ramps 0→20→40→60 VUs over 20 minutes
- **Traffic Model**: 3x normal load
- **Use Case**: Capacity planning, autoscaling validation

### 4. Spike Test
- **Purpose**: Validate system behavior under sudden traffic surge
- **Load**: 10 VUs → sudden spike to 100 VUs → drop back
- **Traffic Model**: Simulates HN/Reddit front page
- **Use Case**: Circuit breaker, rate limiting, queue depth testing

### 5. Soak Test
- **Purpose**: Detect memory leaks, resource exhaustion
- **Load**: 20 VUs sustained for 30 minutes
- **Use Case**: Production readiness, long-term stability

---

## SLO Targets

Tests validate against `docs/SLO.md` targets:

| SLO | Target | K6 Threshold |
|-----|--------|--------------|
| **API Availability** | ≥99.0% | `checks{slo:availability}: rate>=0.99` |
| **P95 Latency (Free)** | ≤12s | `scan_latency{tier:free}: p(95)<12000` |
| **P99 Latency (Free)** | ≤25s | `scan_latency{tier:free}: p(99)<25000` |
| **Queue Wait (Free)** | p95 ≤30s | `queue_wait_time{tier:free}: p(95)<30000` |
| **Error Rate** | <1% | `errors: rate<0.01` |
| **HTTP Failures** | <1% | `http_req_failed: rate<0.01` |

**Pass Criteria**: All thresholds must pass for mainnet launch approval

---

## Results

### Output Files

Results are automatically saved to:
- `results/summary.json` - Machine-readable metrics
- `results/summary.html` - Human-readable report with charts
- `results/k6-{timestamp}.log` - Full execution log

### Interpreting Results

#### ✅ PASS Example
```
✓ checks{slo:availability}........: 99.8%
✓ scan_latency{tier:free}.........: p(95)=11234ms, p(99)=23456ms
✓ queue_wait_time{tier:free}......: p(95)=28000ms
✓ errors..........................: rate=0.2%
```
**Verdict**: System meets all SLO targets → **READY FOR MAINNET**

#### ❌ FAIL Example
```
✗ checks{slo:availability}........: 98.5%  (target: ≥99%)
✗ scan_latency{tier:free}.........: p(95)=15234ms  (target: ≤12s)
✓ queue_wait_time{tier:free}......: p(95)=25000ms
✗ errors..........................: rate=1.5%  (target: <1%)
```
**Verdict**: SLO violations detected → **NOT READY** (see recommendations below)

---

## Troubleshooting

### High Latency (P95 >12s)

**Root Causes**:
- Database slow queries
- Unoptimized analyzer code
- Insufficient CPU/memory
- Network bottlenecks

**Actions**:
1. Profile API with `py-spy` or similar
2. Add database indexes
3. Optimize hot code paths
4. Scale vertically (increase resources)

### High Error Rate (>1%)

**Root Causes**:
- Rate limiting triggered
- Timeout exceeded (504)
- Memory exhaustion (OOM)
- Unhandled exceptions

**Actions**:
1. Check error logs for patterns
2. Increase timeout limits
3. Add error tracking (Sentry)
4. Implement circuit breakers

### Queue Wait Time Exceeded

**Root Causes**:
- Insufficient worker capacity
- Long-running analysis jobs blocking queue
- No job prioritization

**Actions**:
1. Increase concurrent worker count
2. Implement job timeout limits
3. Add priority queue (Pro tier first)
4. Optimize analysis algorithms

### Connection Failures

**Root Causes**:
- Network instability
- DNS issues
- SSL certificate problems
- Firewall blocking

**Actions**:
1. Verify BASE_URL is accessible
2. Check SSL certificate validity
3. Test from different network
4. Use `--insecure-skip-tls-verify` flag (dev only)

---

## CI/CD Integration

### GitHub Actions

Add to `.github/workflows/load-test.yml`:

```yaml
name: Load Tests

on:
  schedule:
    - cron: '0 2 * * 0'  # Weekly Sunday 2 AM
  workflow_dispatch:     # Manual trigger

jobs:
  load-test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      
      - name: Install K6
        run: |
          sudo gpg --no-default-keyring --keyring /usr/share/keyrings/k6-archive-keyring.gpg --keyserver hkp://keyserver.ubuntu.com:80 --recv-keys C5AD17C747E3415A3642D57D77C6C491D6AC1D69
          echo "deb [signed-by=/usr/share/keyrings/k6-archive-keyring.gpg] https://dl.k6.io/deb stable main" | sudo tee /etc/apt/sources.list.d/k6.list
          sudo apt-get update
          sudo apt-get install k6
      
      - name: Run Load Test
        env:
          BASE_URL: ${{ secrets.STAGING_URL }}
          API_KEY: ${{ secrets.STAGING_API_KEY }}
        run: |
          cd frontend/tests/load
          k6 run --env SCENARIO=load api-scan-load-test.js
      
      - name: Upload Results
        if: always()
        uses: actions/upload-artifact@v4
        with:
          name: load-test-results
          path: frontend/tests/load/results/
```

### Pre-Mainnet Checklist

Before tagging mainnet release:

```bash
# 1. Run smoke test
k6 run --env SCENARIO=smoke --env BASE_URL=https://staging.sanctifier.io api-scan-load-test.js

# 2. Run load test
k6 run --env SCENARIO=load --env BASE_URL=https://staging.sanctifier.io api-scan-load-test.js

# 3. Run stress test
k6 run --env SCENARIO=stress --env BASE_URL=https://staging.sanctifier.io api-scan-load-test.js

# 4. Verify all thresholds PASS
grep "✓" results/summary.json

# 5. Document results in mainnet signoff issue
cat results/summary.json >> mainnet-signoff-evidence.md
```

---

## Load Test Report Template

After running tests, document findings:

### Mainnet Load Test Report - {DATE}

**Environment**: Staging  
**Version**: v1.0.0-rc1  
**Scenario**: Load Test (normal mainnet traffic)  
**Duration**: 15 minutes

#### Results

| SLO | Target | Actual | Status |
|-----|--------|--------|--------|
| API Availability | ≥99.0% | 99.5% | ✅ PASS |
| P95 Latency (Free) | ≤12s | 10.2s | ✅ PASS |
| P99 Latency (Free) | ≤25s | 22.1s | ✅ PASS |
| Queue Wait p95 | ≤30s | 18.5s | ✅ PASS |
| Error Rate | <1% | 0.3% | ✅ PASS |

#### Recommendations

- ✅ System meets all SLO targets
- ⚠️ Consider increasing worker pool from 5 to 10 for headroom
- ⚠️ Monitor database connection pool usage (peaked at 85%)
- ✅ No scaling changes required for mainnet launch

**Verdict**: **READY FOR MAINNET** 🚀

---

## References

- [K6 Documentation](https://k6.io/docs/)
- [K6 Cloud](https://k6.io/cloud/) - Optional hosted results
- [Grafana Integration](https://k6.io/docs/results-output/real-time/grafana/)
- `docs/SLO.md` - Service Level Objectives
- Issue #1154 - Load testing implementation
- Issue #1153 - SLO definitions

---

## Contributing

### Adding New Test Scenarios

1. Create new `.js` file in `frontend/tests/load/`
2. Follow k6 script structure (see `api-scan-load-test.js`)
3. Define custom thresholds matching SLOs
4. Document in this README
5. Add to CI/CD pipeline

### Test Data

Sample contracts in `api-scan-load-test.js` cover:
- **Small** (< 100 LOC) - Fast analysis path
- **Medium** (100-500 LOC) - Typical contract complexity
- **Large** (> 500 LOC) - Heavy analysis workload

Add more realistic samples from `contracts/` directory.

---

**Last Updated**: 2026-07-29  
**Maintainer**: Platform Team  
**Related Issues**: #1154, #1153, #1146
