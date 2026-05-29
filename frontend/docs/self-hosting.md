# Self-hosting Sanctifier Frontend

This guide describes the safe defaults and module boundaries to keep when running the frontend in your own infrastructure.

## Runtime boundaries

- `app/api/analyze/route.ts` owns contract upload validation, rate limiting, and process execution.
- `app/lib/report-ingestion.ts` owns JSON parsing and workspace normalization for dashboard rendering.
- `app/lib/scan-progress.ts` owns deterministic scan progress phases for the scanner UI.
- `app/lib/transform.ts` owns schema normalization and finding transformation.

Keeping these boundaries avoids UI regressions when API payloads evolve.

## Environment variables

- `SANCTIFIER_BIN`: path or command name for the Sanctifier CLI binary used by `/api/analyze`.
  - Default: `sanctifier`
  - Example: `SANCTIFIER_BIN=/usr/local/bin/sanctifier`

## Deployment defaults

- Keep upload validation enabled (`.rs` only, UTF-8, size limits).
- Keep rate limiting enabled for `/api/analyze`.
- Keep `runtime = "nodejs"` for the analyze route.
- Keep deterministic progress phases in `scan-progress.ts` for predictable user output and easier debugging.

## Quick verification

From `frontend/`:

```bash
npm ci
npm run build
npm run test:e2e:schema
```

This verifies schema-driven dashboard rendering paths used by uploaded/parsed reports.
