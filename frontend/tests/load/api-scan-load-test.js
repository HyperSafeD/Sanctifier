/**
 * K6 Load Test Script - Sanctifier API Scan Endpoint
 * 
 * Tests the `/api/v1/analyze` endpoint under realistic mainnet-scale load
 * Validates against SLO targets defined in docs/SLO.md
 * 
 * Usage:
 *   # Install k6: https://k6.io/docs/get-started/installation/
 *   
 *   # Run smoke test (low load)
 *   k6 run --env SCENARIO=smoke api-scan-load-test.js
 *   
 *   # Run load test (normal mainnet traffic)
 *   k6 run --env SCENARIO=load api-scan-load-test.js
 *   
 *   # Run stress test (peak traffic)
 *   k6 run --env SCENARIO=stress api-scan-load-test.js
 *   
 *   # Run spike test (sudden traffic surge)
 *   k6 run --env SCENARIO=spike api-scan-load-test.js
 *   
 *   # Custom target
 *   k6 run --env BASE_URL=https://staging.sanctifier.io api-scan-load-test.js
 * 
 * @see https://k6.io/docs/
 * @see docs/SLO.md for target SLOs
 * @see #1154 for implementation details
 */

import http from 'k6/http';
import { check, group, sleep } from 'k6';
import { Rate, Trend, Counter } from 'k6/metrics';
import { SharedArray } from 'k6/data';

// ============================================================================
// Configuration
// ============================================================================

const BASE_URL = __ENV.BASE_URL || 'http://localhost:3000';
const API_KEY = __ENV.API_KEY || 'test-key';
const SCENARIO = __ENV.SCENARIO || 'load';

// Custom metrics
const errorRate = new Rate('errors');
const scanLatency = new Trend('scan_latency');
const queueWaitTime = new Trend('queue_wait_time');
const successfulScans = new Counter('successful_scans');
const failedScans = new Counter('failed_scans');

// ============================================================================
// Test Scenarios
// ============================================================================

const scenarios = {
  // Smoke test: Minimal load to verify functionality
  smoke: {
    executor: 'constant-vus',
    vus: 1,
    duration: '1m',
  },
  
  // Load test: Simulate normal mainnet traffic (10K requests/day = ~7 req/min)
  load: {
    executor: 'ramping-vus',
    startVUs: 0,
    stages: [
      { duration: '2m', target: 10 },  // Ramp up to 10 concurrent users
      { duration: '5m', target: 10 },  // Stay at 10 for 5 minutes
      { duration: '2m', target: 20 },  // Ramp to 20
      { duration: '5m', target: 20 },  // Stay at 20
      { duration: '2m', target: 0 },   // Ramp down
    ],
    gracefulRampDown: '30s',
  },
  
  // Stress test: Push system to limits (3x normal load)
  stress: {
    executor: 'ramping-vus',
    startVUs: 0,
    stages: [
      { duration: '2m', target: 20 },
      { duration: '5m', target: 20 },
      { duration: '2m', target: 40 },
      { duration: '5m', target: 40 },
      { duration: '2m', target: 60 },
      { duration: '5m', target: 60 },
      { duration: '3m', target: 0 },
    ],
    gracefulRampDown: '30s',
  },
  
  // Spike test: Sudden traffic surge (simulates HN/Reddit front page)
  spike: {
    executor: 'ramping-vus',
    startVUs: 0,
    stages: [
      { duration: '1m', target: 10 },   // Normal load
      { duration: '30s', target: 100 }, // Sudden spike
      { duration: '3m', target: 100 },  // Sustained spike
      { duration: '1m', target: 10 },   // Drop back
      { duration: '1m', target: 0 },    // Ramp down
    ],
  },
  
  // Soak test: Sustained load over extended period (reliability check)
  soak: {
    executor: 'constant-vus',
    vus: 20,
    duration: '30m',
  },
};

export const options = {
  scenarios: {
    [SCENARIO]: scenarios[SCENARIO],
  },
  
  // SLO thresholds from docs/SLO.md
  thresholds: {
    // API Availability: ≥99.0% success rate
    'checks{slo:availability}': ['rate>=0.99'],
    
    // Scan Latency (Free tier): p95 ≤12s, p99 ≤25s
    'scan_latency{tier:free}': ['p(95)<12000', 'p(99)<25000'],
    
    // Queue Wait Time (Free tier): p95 ≤30s
    'queue_wait_time{tier:free}': ['p(95)<30000'],
    
    // Error rate: <1%
    'errors': ['rate<0.01'],
    
    // HTTP failures: <1%
    'http_req_failed': ['rate<0.01'],
    
    // Overall p95 latency: <15s
    'http_req_duration': ['p(95)<15000'],
  },
  
  // Summary output
  summaryTrendStats: ['avg', 'min', 'med', 'max', 'p(90)', 'p(95)', 'p(99)'],
};

// ============================================================================
// Test Data - Sample Soroban Contracts
// ============================================================================

const contractSamples = new SharedArray('contracts', function () {
  return [
    // Simple contract (fast analysis)
    {
      name: 'SimpleToken',
      size: 'small',
      source: `
use soroban_sdk::{contract, contractimpl, Env, Symbol};

#[contract]
pub struct SimpleToken;

#[contractimpl]
impl SimpleToken {
    pub fn balance(env: Env, addr: Address) -> i128 {
        env.storage().instance().get(&addr).unwrap_or(0)
    }
    
    pub fn transfer(env: Env, from: Address, to: Address, amount: i128) {
        from.require_auth();
        let from_balance = Self::balance(env.clone(), from.clone());
        let to_balance = Self::balance(env.clone(), to.clone());
        
        env.storage().instance().set(&from, &(from_balance - amount));
        env.storage().instance().set(&to, &(to_balance + amount));
    }
}
`.trim(),
    },
    
    // Medium complexity (typical analysis)
    {
      name: 'AmmPool',
      size: 'medium',
      source: `
use soroban_sdk::{contract, contractimpl, Env, Address, Symbol};

#[contract]
pub struct AmmPool;

#[contractimpl]
impl AmmPool {
    pub fn initialize(env: Env, token_a: Address, token_b: Address, admin: Address) {
        admin.require_auth();
        env.storage().instance().set(&Symbol::new(&env, "token_a"), &token_a);
        env.storage().instance().set(&Symbol::new(&env, "token_b"), &token_b);
        env.storage().instance().set(&Symbol::new(&env, "reserve_a"), &0i128);
        env.storage().instance().set(&Symbol::new(&env, "reserve_b"), &0i128);
    }
    
    pub fn add_liquidity(env: Env, provider: Address, amount_a: i128, amount_b: i128) {
        provider.require_auth();
        let reserve_a: i128 = env.storage().instance().get(&Symbol::new(&env, "reserve_a")).unwrap();
        let reserve_b: i128 = env.storage().instance().get(&Symbol::new(&env, "reserve_b")).unwrap();
        
        // Calculate optimal amounts
        let amount_b_optimal = if reserve_a == 0 {
            amount_b
        } else {
            (amount_a * reserve_b) / reserve_a
        };
        
        env.storage().instance().set(&Symbol::new(&env, "reserve_a"), &(reserve_a + amount_a));
        env.storage().instance().set(&Symbol::new(&env, "reserve_b"), &(reserve_b + amount_b_optimal));
    }
    
    pub fn swap(env: Env, user: Address, amount_in: i128, min_amount_out: i128, a_to_b: bool) -> i128 {
        user.require_auth();
        let reserve_in_key = if a_to_b { Symbol::new(&env, "reserve_a") } else { Symbol::new(&env, "reserve_b") };
        let reserve_out_key = if a_to_b { Symbol::new(&env, "reserve_b") } else { Symbol::new(&env, "reserve_a") };
        
        let reserve_in: i128 = env.storage().instance().get(&reserve_in_key).unwrap();
        let reserve_out: i128 = env.storage().instance().get(&reserve_out_key).unwrap();
        
        // x * y = k formula
        let amount_out = (amount_in * reserve_out) / (reserve_in + amount_in);
        
        if amount_out < min_amount_out {
            panic!("Slippage exceeded");
        }
        
        env.storage().instance().set(&reserve_in_key, &(reserve_in + amount_in));
        env.storage().instance().set(&reserve_out_key, &(reserve_out - amount_out));
        
        amount_out
    }
}
`.trim(),
    },
    
    // Large contract (slow analysis)
    {
      name: 'Governance',
      size: 'large',
      source: `
use soroban_sdk::{contract, contractimpl, Env, Address, Symbol, Vec, Map};

#[derive(Clone)]
pub struct Proposal {
    pub id: u64,
    pub proposer: Address,
    pub title: Symbol,
    pub description: Symbol,
    pub for_votes: i128,
    pub against_votes: i128,
    pub start_time: u64,
    pub end_time: u64,
    pub executed: bool,
}

#[contract]
pub struct Governance;

#[contractimpl]
impl Governance {
    pub fn initialize(env: Env, admin: Address, voting_token: Address, quorum: i128) {
        admin.require_auth();
        env.storage().instance().set(&Symbol::new(&env, "admin"), &admin);
        env.storage().instance().set(&Symbol::new(&env, "voting_token"), &voting_token);
        env.storage().instance().set(&Symbol::new(&env, "quorum"), &quorum);
        env.storage().instance().set(&Symbol::new(&env, "proposal_count"), &0u64);
    }
    
    pub fn create_proposal(env: Env, proposer: Address, title: Symbol, description: Symbol, duration: u64) -> u64 {
        proposer.require_auth();
        
        let count: u64 = env.storage().instance().get(&Symbol::new(&env, "proposal_count")).unwrap();
        let proposal_id = count + 1;
        
        let proposal = Proposal {
            id: proposal_id,
            proposer: proposer.clone(),
            title,
            description,
            for_votes: 0,
            against_votes: 0,
            start_time: env.ledger().timestamp(),
            end_time: env.ledger().timestamp() + duration,
            executed: false,
        };
        
        env.storage().instance().set(&Symbol::new(&env, &format!("proposal_{}", proposal_id)), &proposal);
        env.storage().instance().set(&Symbol::new(&env, "proposal_count"), &proposal_id);
        
        proposal_id
    }
    
    pub fn vote(env: Env, voter: Address, proposal_id: u64, support: bool, weight: i128) {
        voter.require_auth();
        
        let mut proposal: Proposal = env.storage().instance()
            .get(&Symbol::new(&env, &format!("proposal_{}", proposal_id)))
            .unwrap();
        
        if env.ledger().timestamp() > proposal.end_time {
            panic!("Voting period ended");
        }
        
        if support {
            proposal.for_votes += weight;
        } else {
            proposal.against_votes += weight;
        }
        
        env.storage().instance().set(&Symbol::new(&env, &format!("proposal_{}", proposal_id)), &proposal);
        env.storage().instance().set(&Symbol::new(&env, &format!("voted_{}_{}", proposal_id, voter)), &true);
    }
    
    pub fn execute(env: Env, executor: Address, proposal_id: u64) {
        executor.require_auth();
        
        let mut proposal: Proposal = env.storage().instance()
            .get(&Symbol::new(&env, &format!("proposal_{}", proposal_id)))
            .unwrap();
        
        if env.ledger().timestamp() <= proposal.end_time {
            panic!("Voting period not ended");
        }
        
        let quorum: i128 = env.storage().instance().get(&Symbol::new(&env, "quorum")).unwrap();
        let total_votes = proposal.for_votes + proposal.against_votes;
        
        if total_votes < quorum {
            panic!("Quorum not reached");
        }
        
        if proposal.for_votes <= proposal.against_votes {
            panic!("Proposal rejected");
        }
        
        proposal.executed = true;
        env.storage().instance().set(&Symbol::new(&env, &format!("proposal_{}", proposal_id)), &proposal);
    }
}
`.trim(),
    },
  ];
});

// ============================================================================
// Test Functions
// ============================================================================

/**
 * Main test scenario: Analyze a contract
 */
export default function () {
  // Select a random contract sample
  const contract = contractSamples[Math.floor(Math.random() * contractSamples.length)];
  
  group('API Scan Request', function () {
    const startTime = Date.now();
    
    const payload = JSON.stringify({
      source: contract.source,
      options: {
        rules: ['all'],
        format: 'json',
      },
    });
    
    const params = {
      headers: {
        'Content-Type': 'application/json',
        'x-api-key': API_KEY,
      },
      tags: {
        name: 'analyze_contract',
        contract_size: contract.size,
        tier: 'free',
      },
    };
    
    // Make request
    const response = http.post(`${BASE_URL}/api/v1/analyze`, payload, params);
    
    // Calculate latency
    const latency = Date.now() - startTime;
    scanLatency.add(latency, { tier: 'free', size: contract.size });
    
    // Check response
    const success = check(response, {
      'status is 200 or 202': (r) => r.status === 200 || r.status === 202,
      'status is not 5xx': (r) => r.status < 500,
      'response has body': (r) => r.body.length > 0,
      'response is valid JSON': (r) => {
        try {
          JSON.parse(r.body);
          return true;
        } catch {
          return false;
        }
      },
    }, { slo: 'availability' });
    
    if (success) {
      successfulScans.add(1);
      
      // Parse response to check for queue wait time
      try {
        const data = JSON.parse(response.body);
        if (data.queue_wait_ms) {
          queueWaitTime.add(data.queue_wait_ms, { tier: 'free' });
        }
      } catch (e) {
        // Ignore parse errors
      }
    } else {
      failedScans.add(1);
      errorRate.add(1);
      console.error(`Request failed: ${response.status} - ${response.body.substring(0, 200)}`);
    }
    
    // Check specific status codes
    check(response, {
      'not rate limited (429)': (r) => r.status !== 429,
      'not timeout (504)': (r) => r.status !== 504,
      'not payload too large (413)': (r) => r.status !== 413,
    });
  });
  
  // Simulate user think time (reading results)
  sleep(Math.random() * 3 + 2); // 2-5 seconds
}

/**
 * Setup function - runs once before all VUs start
 */
export function setup() {
  console.log(`Starting load test: ${SCENARIO}`);
  console.log(`Target: ${BASE_URL}`);
  console.log(`VUs: ${JSON.stringify(scenarios[SCENARIO])}`);
  
  // Warm-up request
  const warmup = http.get(`${BASE_URL}/api/health`);
  console.log(`Warmup request: ${warmup.status}`);
  
  return { timestamp: new Date().toISOString() };
}

/**
 * Teardown function - runs once after all VUs finish
 */
export function teardown(data) {
  console.log(`Load test completed at ${new Date().toISOString()}`);
  console.log(`Test started at ${data.timestamp}`);
}

/**
 * Custom summary output
 */
export function handleSummary(data) {
  return {
    'stdout': textSummary(data, { indent: ' ', enableColors: true }),
    'frontend/tests/load/results/summary.json': JSON.stringify(data, null, 2),
    'frontend/tests/load/results/summary.html': htmlReport(data),
  };
}

// ============================================================================
// Helper Functions
// ============================================================================

function textSummary(data, options) {
  // K6 built-in summary
  return require('https://jslib.k6.io/k6-summary/0.0.2/index.js').textSummary(data, options);
}

function htmlReport(data) {
  const timestamp = new Date().toISOString();
  const passedThresholds = Object.entries(data.metrics)
    .filter(([_, metric]) => metric.thresholds && Object.values(metric.thresholds).every(t => t.ok))
    .length;
  const totalThresholds = Object.entries(data.metrics)
    .filter(([_, metric]) => metric.thresholds)
    .length;
  
  return `
<!DOCTYPE html>
<html>
<head>
  <title>K6 Load Test Results - ${timestamp}</title>
  <style>
    body { font-family: Arial, sans-serif; margin: 20px; background: #f5f5f5; }
    .container { max-width: 1200px; margin: 0 auto; background: white; padding: 30px; border-radius: 8px; box-shadow: 0 2px 4px rgba(0,0,0,0.1); }
    h1 { color: #333; border-bottom: 3px solid #007bff; padding-bottom: 10px; }
    .summary { display: grid; grid-template-columns: repeat(auto-fit, minmax(250px, 1fr)); gap: 20px; margin: 30px 0; }
    .metric-card { background: #f8f9fa; padding: 20px; border-radius: 6px; border-left: 4px solid #007bff; }
    .metric-card h3 { margin: 0 0 10px 0; font-size: 14px; color: #666; text-transform: uppercase; }
    .metric-card .value { font-size: 32px; font-weight: bold; color: #333; }
    .metric-card .unit { font-size: 14px; color: #999; }
    .threshold-pass { color: #28a745; }
    .threshold-fail { color: #dc3545; }
    table { width: 100%; border-collapse: collapse; margin: 20px 0; }
    th, td { padding: 12px; text-align: left; border-bottom: 1px solid #ddd; }
    th { background: #007bff; color: white; font-weight: 600; }
    tr:hover { background: #f8f9fa; }
    .pass { color: #28a745; font-weight: bold; }
    .fail { color: #dc3545; font-weight: bold; }
    .footer { margin-top: 40px; padding-top: 20px; border-top: 1px solid #ddd; color: #666; font-size: 12px; }
  </style>
</head>
<body>
  <div class="container">
    <h1>📊 Sanctifier Load Test Results</h1>
    <p><strong>Scenario:</strong> ${SCENARIO} | <strong>Timestamp:</strong> ${timestamp}</p>
    
    <div class="summary">
      <div class="metric-card">
        <h3>Total Requests</h3>
        <div class="value">${data.metrics.http_reqs?.values.count || 0}</div>
      </div>
      <div class="metric-card">
        <h3>Success Rate</h3>
        <div class="value">${((1 - (data.metrics.errors?.values.rate || 0)) * 100).toFixed(2)}<span class="unit">%</span></div>
      </div>
      <div class="metric-card">
        <h3>Avg Latency</h3>
        <div class="value">${(data.metrics.http_req_duration?.values.avg || 0).toFixed(0)}<span class="unit">ms</span></div>
      </div>
      <div class="metric-card">
        <h3>P95 Latency</h3>
        <div class="value">${(data.metrics.http_req_duration?.values['p(95)'] || 0).toFixed(0)}<span class="unit">ms</span></div>
      </div>
      <div class="metric-card">
        <h3>Thresholds</h3>
        <div class="value ${passedThresholds === totalThresholds ? 'threshold-pass' : 'threshold-fail'}">${passedThresholds}/${totalThresholds}</div>
      </div>
    </div>
    
    <h2>📈 Detailed Metrics</h2>
    <table>
      <thead>
        <tr>
          <th>Metric</th>
          <th>Avg</th>
          <th>Min</th>
          <th>Max</th>
          <th>P95</th>
          <th>P99</th>
        </tr>
      </thead>
      <tbody>
        ${Object.entries(data.metrics)
          .filter(([_, metric]) => metric.type === 'trend')
          .map(([name, metric]) => `
            <tr>
              <td>${name}</td>
              <td>${metric.values.avg.toFixed(2)}</td>
              <td>${metric.values.min.toFixed(2)}</td>
              <td>${metric.values.max.toFixed(2)}</td>
              <td>${metric.values['p(95)'].toFixed(2)}</td>
              <td>${metric.values['p(99)'].toFixed(2)}</td>
            </tr>
          `).join('')}
      </tbody>
    </table>
    
    <h2>✅ SLO Compliance</h2>
    <table>
      <thead>
        <tr>
          <th>SLO</th>
          <th>Target</th>
          <th>Actual</th>
          <th>Status</th>
        </tr>
      </thead>
      <tbody>
        <tr>
          <td>API Availability</td>
          <td>≥99%</td>
          <td>${((1 - (data.metrics.errors?.values.rate || 0)) * 100).toFixed(2)}%</td>
          <td class="${(1 - (data.metrics.errors?.values.rate || 0)) >= 0.99 ? 'pass' : 'fail'}">${(1 - (data.metrics.errors?.values.rate || 0)) >= 0.99 ? '✅ PASS' : '❌ FAIL'}</td>
        </tr>
        <tr>
          <td>P95 Scan Latency (Free)</td>
          <td>≤12s</td>
          <td>${((data.metrics.scan_latency?.values['p(95)'] || 0) / 1000).toFixed(2)}s</td>
          <td class="${(data.metrics.scan_latency?.values['p(95)'] || 0) <= 12000 ? 'pass' : 'fail'}">${(data.metrics.scan_latency?.values['p(95)'] || 0) <= 12000 ? '✅ PASS' : '❌ FAIL'}</td>
        </tr>
        <tr>
          <td>P99 Scan Latency (Free)</td>
          <td>≤25s</td>
          <td>${((data.metrics.scan_latency?.values['p(99)'] || 0) / 1000).toFixed(2)}s</td>
          <td class="${(data.metrics.scan_latency?.values['p(99)'] || 0) <= 25000 ? 'pass' : 'fail'}">${(data.metrics.scan_latency?.values['p(99)'] || 0) <= 25000 ? '✅ PASS' : '❌ FAIL'}</td>
        </tr>
      </tbody>
    </table>
    
    <div class="footer">
      Generated by K6 Load Test Suite | See docs/SLO.md for target definitions | Issue #1154
    </div>
  </div>
</body>
</html>
  `.trim();
}
