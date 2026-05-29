# WASM Security Hardening (CSP-friendly)

This document describes the security measures implemented in `@sanctifier/wasm` to ensure it is safe for high-security environments and compatible with strict Content Security Policies (CSP).

## Security Goals

1.  **Zero `eval()` Usage**: The module must not use the `eval()` function, which is a common vector for XSS and is blocked by many CSPs.
2.  **No `new Function()`**: The module must not use `new Function(...)` (dynamic code generation), which also triggers CSP `unsafe-eval` violations.
3.  **Strict CSP Compliance**: The module should function perfectly under a CSP header that does not include `'unsafe-eval'`.

## Implementation Details

### Build-time Verification

A dedicated security audit script, `verify-csp-compliance.js`, runs during every CI build. This script performs a byte-level scan of the generated JavaScript glue code to detect any CSP-violating patterns.

**Patterns Monitored:**
- `eval(...)`
- `new Function(...)`

### Dependency Hardening

The package uses `wasm-bindgen` (version 0.2.84 or later) configured with `--target web`. Modern versions of `wasm-bindgen` avoid legacy dynamic code generation patterns used for global environment detection.

### Automated CI Verification

The GitHub Actions workflow (`ci.yml`) includes a mandatory "Verify CSP Compliance" step. If any violation is detected in the `pkg/` output, the build fails immediately.

```yaml
- name: Verify CSP Compliance
  run: |
    cd tooling/sanctifier-wasm
    node scripts/verify-csp-compliance.js
```

## Testing for Compliance

### E2E Security Tests

We provide an automated E2E test suite using Playwright (`frontend/tests/e2e/csp-security.spec.ts`) that:
1.  Intercepts browser requests to inject a strict CSP header (`script-src 'self'`).
2.  Loads the WASM module inside the browser environment.
3.  Verifies that no CSP violations are logged to the browser console.
4.  Ensures the module correctly initializes and executes.

### Manual Verification

To run the local audit manually:

```bash
cd tooling/sanctifier-wasm
node scripts/verify-csp-compliance.js
```

## Deployment Recommendations

When deploying the Sanctifier frontend, we recommend the following CSP header:

```http
Content-Security-Policy: default-src 'self'; script-src 'self'; object-src 'none';
```

The `@sanctifier/wasm` module is fully compatible with this configuration and does **not** require `unsafe-eval`.
