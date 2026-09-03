# Sanctifier Telemetry

Sanctifier does not collect telemetry unless you explicitly enable it.

## What is sent

When telemetry is enabled, Sanctifier submits only:

- Rule IDs that matched during the scan
- Total analysis duration in milliseconds
- Sanitized tool version

It does not send:

- Source code
- File paths
- Contract names
- Repository URLs
- Secret keys or credentials

## How to enable

You can opt in with:

```bash
sanctifier init --telemetry on
```

or by setting the config file value:

```toml
telemetry = true
```

If you want to send telemetry to a private collector, set:

```bash
export SANCTIFIER_TELEMETRY_URL="https://your-collector.example/ingest"
```

## How to opt out

Telemetry is off by default. To disable it:

- Set `telemetry = false` in `.sanctify.toml`
- Or rerun `sanctifier init --telemetry off`
- Or unset `SANCTIFIER_TELEMETRY_URL`

## Privacy Notes

The telemetry payload is intentionally minimal so you can track feature usage without exposing analyzed code. If you do not set `SANCTIFIER_TELEMETRY_URL`, Sanctifier keeps the telemetry payload local and does not send it anywhere.
