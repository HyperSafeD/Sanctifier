# Sanctifier Hosted API — Pricing & Limits

## API Endpoint

**POST** `/api/v1/analyze`

Submit a Soroban smart contract source file for automated security analysis.

## Authentication

All requests require an API key sent via the `x-api-key` header.

```bash
curl -X POST https://sanctifier.hyperfused.xyz/api/v1/analyze \
  -H "x-api-key: sk-..." \
  -H "Content-Type: application/json" \
  -d '{"source": "use soroban_sdk; ..."}'
```

## Rate Limits

| Tier       | Requests / min | Max file size | Timeout | Concurrent |
|------------|---------------|---------------|---------|------------|
| Free       | 10            | 100 KB        | 15 s    | 1          |
| Pro        | 60            | 500 KB        | 60 s    | 5          |
| Enterprise | Unlimited     | 2 MB          | 120 s   | 20         |

Rate limits are enforced per API key using a sliding-window counter. Exceeded requests receive HTTP `429 Too Many Requests` with a `Retry-After` header.

## Response Format

The API returns JSON by default. Append `?format=sarif` to receive SARIF v2.1.0 output.

### JSON Response (default)

```json
{
  "success": true,
  "summary": {
    "total_findings": 5,
    "critical": 1,
    "high": 2,
    "medium": 1,
    "low": 1
  },
  "findings": [
    {
      "id": "auth-0",
      "code": "S001",
      "severity": "critical",
      "category": "Auth Gap",
      "title": "Missing require_auth()",
      "location": "contract.rs:transfer",
      "suggestion": "Add require_auth() call before state mutation"
    }
  ]
}
```

### SARIF Response (`?format=sarif`)

```json
{
  "$schema": "https://raw.githubusercontent.com/oasis-tcs/sarif-spec/master/Schemata/sarif-schema-2.1.0.json",
  "version": "2.1.0",
  "runs": [
    {
      "tool": { "driver": { "name": "Sanctifier", "rules": [...] } },
      "results": [...]
    }
  ]
}
```

## Error Codes

| HTTP Status | Code                         | Description                          |
|-------------|------------------------------|--------------------------------------|
| 400         | INVALID_INPUT                | Missing or malformed request body    |
| 401         | UNAUTHORIZED                 | Missing or invalid API key           |
| 413         | PAYLOAD_TOO_LARGE            | Source file exceeds size limit       |
| 422         | UNSUPPORTED_CONTRACT         | Source is not a valid Soroban contract |
| 429         | RATE_LIMIT_EXCEEDED          | Too many requests                    |
| 500         | ANALYSIS_FAILED              | Internal analysis error              |
| 504         | ANALYSIS_TIMEOUT             | Analysis exceeded time limit         |

## Local Development

To run the API locally:

```bash
# Set up API keys
export API_KEYS="sk-test-key-1,sk-test-key-2"
export API_RATE_LIMIT_PER_MINUTE=100

# Start the Next.js dev server
cd frontend
npm run dev
```

Then test:

```bash
curl -X POST http://localhost:3000/api/v1/analyze \
  -H "x-api-key: sk-test-key-1" \
  -H "Content-Type: application/json" \
  -d '{"source": "use soroban_sdk; contract Contract; impl Contract { pub fn add(a: u64, b: u64) -> u64 { a + b } }"}'
```
