# Load Testing Implementation Notes

**Issue**: #1154  
**Implemented By**: @dev-susa  
**Date**: 2026-07-29  
**Status**: ✅ Infrastructure Complete, Ready for Production Testing

---

## What Was Implemented

### 1. K6 Load Testing Script (`api-scan-load-test.js`)

Comprehensive load testing script featuring:

#### Test Scenarios
- ✅ **Smoke Test** (1 VU, 1 min) - Quick sanity check
- ✅ **Load Test** (10-20 VUs, 15 min) - Normal mainnet traffic
- ✅ **Stress Test** (20-60 VUs, 20 min) - 3x normal load
- ✅ **Spike Test** (10→100 VUs) - Sudden traffic surge
- ✅ **Soak Test** (20 VUs, 30 min) - Long-term stability

#### Sample Contracts
Three realistic Soroban contracts with varying complexity:
- **SimpleToken** (~50 LOC) - Fast analysis path
- **AmmPool** (~150 LOC) - Medium complexity
- **Governance** (~250 LOC) - Heavy workload

#### SLO Validation
Automated thresholds matching `docs/SLO.md`:
- API Availability: ≥99.0%
- P95 Latency (Free): ≤12s
- P99 Latency (Free): ≤25s
- Queue Wait p95: ≤30s
- Error Rate: <1%

#### Custom Metrics
- `scan_latency` - End-to-end scan time (tagged by tier)
- `queue_wait_time` - Time spent in job queue
- `successful_scans` - Counter of successful requests
- `failed_scans` - Counter of failed requests
- `errors` - Error rate (used for SLO validation)

#### Output Formats
- **JSON** - Machine-readable metrics
- **HTML** - Beautiful report with charts and SLO compliance table
- **Console** - Real-time progress and summary

### 2. Documentation (`README.md`)

Complete user guide covering:
- Installation instructions (macOS, Linux, Windows, Docker)
- Quick start examples
- Scenario descriptions
- SLO targets and pass criteria
- Result interpretation
- Troubleshooting guide
- CI/CD integration examples
- Pre-mainnet checklist

### 3. Automation Script (`run-all-tests.sh`)

Bash script to run all scenarios sequentially:
- Environment selection (staging/production/localhost)
- Automatic smoke test → load test → stress test → spike test
- Combined reporting
- Pass/fail verdict
- Safety check for production testing

### 4. Results Directory

Organized output structure:
```
results/
├── {timestamp}/
│   ├── smoke-output.json
│   ├── smoke-console.log
│   ├── load-output.json
│   ├── load-console.log
│   ├── stress-output.json
│   ├── stress-console.log
│   ├── spike-output.json
│   └── spike-console.log
└── .gitignore (excludes results from git)
```

---

## Usage

### Prerequisites

Install k6:
```bash
# macOS
brew install k6

# Linux
curl https://github.com/grafana/k6/releases/download/v0.47.0/k6-v0.47.0-linux-amd64.tar.gz -L | tar xvz
sudo mv k6-v0.47.0-linux-amd64/k6 /usr/local/bin

# Docker
docker pull grafana/k6
```

### Quick Test

```bash
cd frontend/tests/load

# Run smoke test (1 minute)
k6 run --env SCENARIO=smoke api-scan-load-test.js

# Run load test (15 minutes)
k6 run --env SCENARIO=load api-scan-load-test.js
```

### Full Test Suite

```bash
# Run all scenarios against staging
./run-all-tests.sh staging

# Results saved to: results/{timestamp}/
```

### Custom Configuration

```bash
# Test against localhost
k6 run --env BASE_URL=http://localhost:3000 --env SCENARIO=smoke api-scan-load-test.js

# Test with custom API key
k6 run --env API_KEY=sk-your-key-here api-scan-load-test.js

# Save results to custom location
k6 run api-scan-load-test.js --out json=custom-results.json
```

---

## SLO Compliance Validation

The script automatically validates against SLO targets:

### Example: PASS
```
✓ checks{slo:availability}.........: 99.8% (target: ≥99%)
✓ scan_latency{tier:free}..........: p(95)=10234ms (target: ≤12s)
✓ scan_latency{tier:free}..........: p(99)=22456ms (target: ≤25s)
✓ queue_wait_time{tier:free}.......: p(95)=25000ms (target: ≤30s)
✓ errors...........................: rate=0.2% (target: <1%)

Verdict: ✅ PASS - Ready for mainnet
```

### Example: FAIL
```
✗ checks{slo:availability}.........: 98.3% (target: ≥99%)
✗ scan_latency{tier:free}..........: p(95)=15234ms (target: ≤12s)
✓ queue_wait_time{tier:free}.......: p(95)=28000ms (target: ≤30s)
✗ errors...........................: rate=1.7% (target: <1%)

Verdict: ❌ FAIL - NOT READY for mainnet
Actions:
  1. Investigate high error rate (check logs)
  2. Optimize slow scan latency (profile code)
  3. Scale resources if needed
  4. Re-run tests after fixes
```

---

## Next Steps

### Before Mainnet Launch

1. **Deploy to Staging**
   ```bash
   # Ensure staging environment is running
   curl https://staging.sanctifier.io/api/health
   ```

2. **Run Full Test Suite**
   ```bash
   cd frontend/tests/load
   ./run-all-tests.sh staging
   ```

3. **Verify SLO Compliance**
   - All thresholds must PASS
   - Review HTML reports for anomalies
   - Document results in mainnet signoff issue

4. **Load Test Against Production** (post-launch)
   ```bash
   # After mainnet launch, validate production performance
   ./run-all-tests.sh production
   ```

### CI/CD Integration

Add to `.github/workflows/load-test.yml`:

```yaml
name: Weekly Load Tests

on:
  schedule:
    - cron: '0 2 * * 0'  # Every Sunday at 2 AM UTC
  workflow_dispatch:

jobs:
  load-test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      
      - name: Install K6
        run: |
          curl https://github.com/grafana/k6/releases/download/v0.47.0/k6-v0.47.0-linux-amd64.tar.gz -L | tar xvz
          sudo mv k6-v0.47.0-linux-amd64/k6 /usr/local/bin
      
      - name: Run Load Tests
        env:
          BASE_URL: ${{ secrets.STAGING_URL }}
          API_KEY: ${{ secrets.STAGING_API_KEY }}
        run: |
          cd frontend/tests/load
          ./run-all-tests.sh staging
      
      - name: Upload Results
        if: always()
        uses: actions/upload-artifact@v4
        with:
          name: load-test-results
          path: frontend/tests/load/results/
      
      - name: Comment on Issue
        if: failure()
        uses: actions/github-script@v7
        with:
          script: |
            github.rest.issues.createComment({
              issue_number: 1154,
              owner: context.repo.owner,
              repo: context.repo.name,
              body: '❌ Weekly load test failed! See workflow run for details.'
            })
```

---

## Troubleshooting

### "Connection Refused" Error

**Cause**: BASE_URL is not accessible

**Solution**:
```bash
# Verify URL is reachable
curl -I https://staging.sanctifier.io

# Check if local dev server is running
cd frontend && npm run dev
```

### "API Key Invalid" Error

**Cause**: Missing or invalid API_KEY

**Solution**:
```bash
# Set API key environment variable
export API_KEY=sk-your-staging-key

# Or pass inline
k6 run --env API_KEY=sk-your-key api-scan-load-test.js
```

### High Memory Usage

**Cause**: K6 running with too many VUs

**Solution**:
```bash
# Reduce VU count in script or run smoke test
k6 run --env SCENARIO=smoke api-scan-load-test.js

# Or use smaller scenario
k6 run --vus 5 --duration 2m api-scan-load-test.js
```

### Slow Test Execution

**Cause**: Network latency or slow API responses

**Expected**: Load test should take 15-20 minutes for full suite

**Solution**:
- Run smoke test first (1 minute) to verify connectivity
- Check API performance separately with `curl`
- Consider running tests closer to staging environment

---

## Limitations & Future Work

### Current Limitations

1. **API Only** - Dashboard UI load testing not yet implemented
2. **Single Endpoint** - Only tests `/api/v1/analyze`
3. **Synthetic Data** - Uses hardcoded contract samples
4. **No Authentication Tiers** - Only tests Free tier (needs Pro/Enterprise scenarios)

### Future Enhancements

Track in follow-up issues:

- [ ] **Dashboard Load Test** - Simulate concurrent users browsing results
- [ ] **Mixed Workflow Test** - Combined API + dashboard user journeys
- [ ] **Real Contract Corpus** - Load test with actual production contracts
- [ ] **Pro/Enterprise Tiers** - Separate scenarios for paid tiers with higher concurrency
- [ ] **Geographic Distribution** - Test from multiple regions (US-East, EU, APAC)
- [ ] **Grafana Integration** - Real-time metrics dashboard during tests
- [ ] **Automated Regression Detection** - Compare results against baseline
- [ ] **Cost Estimation** - Calculate infrastructure cost at scale

---

## Cost Considerations

### K6 Cloud (Optional)

K6 offers a hosted platform for:
- Distributed load testing from multiple regions
- Real-time metrics and dashboards
- Historical result comparison
- Team collaboration features

**Pricing**: Free tier available, paid plans start at $49/month

**Recommendation**: Start with local k6, upgrade to Cloud if team needs collaboration features

### Infrastructure Scaling

Based on load test results, estimate required infrastructure:

| Scenario | VUs | Expected RPS | Est. Cost/Month (Vercel) |
|----------|-----|--------------|--------------------------|
| Smoke | 1 | ~0.5 | $0 (hobby tier) |
| Load | 10-20 | ~5-10 | $20 (pro tier) |
| Stress | 60 | ~30 | $100 (with autoscaling) |
| Production | 100+ | ~50+ | $200-500 (enterprise) |

*Note: Actual costs depend on request size, analysis complexity, and caching*

---

## References

- **K6 Documentation**: https://k6.io/docs/
- **SLO Targets**: `docs/SLO.md`
- **Issue**: #1154
- **Related**: #1153 (SLO definitions), #1146 (RPC resilience)

---

## Acceptance Criteria Status

- [x] Load test suite implemented and runnable ✅
- [x] K6 scripts with realistic traffic patterns ✅
- [x] SLO validation thresholds configured ✅
- [x] HTML/JSON reporting ✅
- [x] Documentation complete ✅
- [ ] Results documented against SLO targets ⏳ (pending first run)
- [ ] Pass/fail verdict for mainnet-readiness ⏳ (pending first run)

**Status**: Infrastructure complete, ready for first production test run

---

**Last Updated**: 2026-07-29  
**Maintainer**: @dev-susa  
**Next Review**: After first staging run
