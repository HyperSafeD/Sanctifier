# Sanctifier REST API — Pricing & Limits

## Overview

The Sanctifier REST API provides on-demand security analysis for Soroban smart contracts. This page documents rate limits, quotas, and pricing information.

## Authentication

All API requests must include a valid API key:

```bash
curl -X POST https://api.sanctifier.hypersafeD.io/v1/analyze \
  -H "x-api-key: your-api-key-here" \
  -H "Content-Type: application/json" \
  -d '{"source": "..."}'
```

Alternative header names:
- `x-api-key`
- `Authorization: Bearer <key>`

## Rate Limiting

### Free Tier
- **Requests per minute:** 5
- **Requests per day:** 100
- **Max file size:** 250 KB
- **Execution timeout:** 30 seconds
- **Concurrent jobs:** 1

### Pro Tier (Coming Soon)
- **Requests per minute:** 20
- **Requests per day:** 1,000
- **Max file size:** 1 MB
- **Execution timeout:** 60 seconds
- **Concurrent jobs:** 5

### Enterprise
Contact [security@hypersafeD.io](mailto:security@hypersafeD.io) for custom limits.

## Rate Limit Headers

All API responses include rate-limit information:

```
x-ratelimit-limit: 5
x-ratelimit-remaining: 4
x-ratelimit-reset: 1685739200
retry-after: 30
```

When rate-limited, the API responds with HTTP 429:

```json
{
  "error": "Rate limit exceeded. Please try again later.",
  "retry-after": 30
}
```

## File Size Limits

- **JSON request body:** 250 KB max
- **Multipart file upload:** 250 KB max

Requests exceeding this limit receive HTTP 413:

```json
{
  "error": "Source exceeds limit of 250 KB."
}
```

## Timeouts

- **Analysis execution:** 30 seconds per request
- **API response:** 60 seconds

Requests taking longer than 30 seconds receive HTTP 504:

```json
{
  "error": "Analysis timed out after 30 seconds"
}
```

## Supported Contract Types

Only Soroban smart contracts are supported. The source code must import:
- `soroban_sdk`
- `soroban-sdk`

Non-Soroban contracts receive HTTP 422:

```json
{
  "error": "Source is not a Soroban contract (missing soroban-sdk import)."
}
```

## Response Formats

The API supports two response formats, controlled by the `format` query parameter:

### JSON (Default)

```bash
POST /api/v1/analyze?format=json
```

Response includes findings array and metadata:

```json
{
  "success": true,
  "summary": {
    "total_findings": 5,
    "critical": 1,
    "high": 2,
    "medium": 2,
    "low": 0
  },
  "findings": [...],
  "report": {...}
}
```

### SARIF Format

```bash
POST /api/v1/analyze?format=sarif
```

Returns SARIF 2.1 (Static Analysis Results Interchange Format) compatible with CI/CD tools:

```json
{
  "version": "2.1.0",
  "$schema": "https://raw.githubusercontent.com/oasis-tcs/sarif-spec/master/Schemata/sarif-schema-2.1.0.json",
  "runs": [...]
}
```

## Quota Resets

Rate limits reset every 60 seconds (UTC). Daily limits reset at midnight UTC.

Track your usage via the API:

```bash
GET /api/v1/analyze
```

Returns service info including current limits.

## Examples

### JSON Request

```bash
curl -X POST https://api.sanctifier.hypersafeD.io/v1/analyze \
  -H "x-api-key: sk_test_abc123" \
  -H "Content-Type: application/json" \
  -d '{
    "source": "use soroban_sdk::{contract, contractimpl, Env};\n\n#[contract]\npub struct MyContract;\n\n#[contractimpl]\nimpl MyContract {\n  pub fn hello(env: Env) -> u32 {\n    42\n  }\n}\n"
  }'
```

### Multipart File Upload

```bash
curl -X POST https://api.sanctifier.hypersafeD.io/v1/analyze \
  -H "x-api-key: sk_test_abc123" \
  -F "contract=@my_contract.rs"
```

### SARIF Output

```bash
curl -X POST https://api.sanctifier.hypersafeD.io/v1/analyze?format=sarif \
  -H "x-api-key: sk_test_abc123" \
  -H "Content-Type: application/json" \
  -d '{"source": "..."}'
```

## Error Codes

| Code | Status | Description |
|------|--------|-------------|
| `unauthorized` | 401 | Missing or invalid API key |
| `rate_limited` | 429 | Rate limit exceeded |
| `invalid_content_type` | 400 | Invalid Content-Type header |
| `invalid_format` | 400 | Invalid `format` parameter |
| `missing_source` | 400 | Missing `source` in request body |
| `file_too_large` | 413 | Source code exceeds size limit |
| `invalid_file_type` | 400 | Only `.rs` files supported |
| `not_soroban` | 422 | Not a Soroban contract |
| `timeout` | 504 | Analysis exceeded time limit |
| `internal_error` | 500 | Server-side analysis failure |

## Best Practices

1. **Batch requests efficiently:** Analyze multiple contracts sequentially to stay within limits
2. **Implement backoff:** Use `retry-after` header before retrying 429 responses
3. **Cache results:** Store analysis results to avoid repeated requests
4. **Use SARIF for CI/CD:** Integrate with GitHub/GitLab workflows via SARIF format
5. **Monitor quotas:** Track requests and implement client-side throttling

## Support

For questions or to request higher limits:
- **Email:** [api-support@hypersafeD.io](mailto:api-support@hypersafeD.io)
- **GitHub Issues:** [HyperSafeD/Sanctifier](https://github.com/HyperSafeD/Sanctifier/issues)
- **Documentation:** [Sanctifier Docs](https://github.com/HyperSafeD/Sanctifier/tree/main/docs)
