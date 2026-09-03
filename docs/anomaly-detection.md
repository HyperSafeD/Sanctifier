# Anomaly Detection Service

`tooling/sanctifier-detector` is the off-chain service for recorded runtime-call monitoring. It polls an event-indexer endpoint, keeps a rolling history of `CallRecord` entries, evaluates the configured rules, and sends Slack or Discord alerts when an anomaly is detected.

## What it watches

The detector ships with four rules:

1. Sudden failure-rate spike
1. Function spam from the same caller
1. Privileged `pause` / `resume` calls from non-admin addresses
1. Off-hours activity for contracts with configured operating windows

## Configuration

Create a JSON config file:

```json
{
  "events_url": "https://indexer.example.com/call-records",
  "poll_interval_seconds": 30,
  "webhook_urls": [
    "https://hooks.slack.com/services/...",
    "https://discord.com/api/webhooks/..."
  ],
  "admins": [
    "GBZXAMPLEADMINADDRESS"
  ],
  "off_hours_windows": {
    "C0123456789ABCDEF0123456789ABCDEF": [
      { "start_hour": 8, "end_hour": 18 }
    ]
  }
}
```

`off_hours_windows` uses UTC hours and treats the end hour as exclusive.

## Running it

```bash
cargo run -p sanctifier-detector -- --config detector.json
```

For a single fetch-and-evaluate pass:

```bash
cargo run -p sanctifier-detector -- --config detector.json --once
```

## Event shape

The detector accepts either a raw JSON array of records or an envelope with a top-level `records` field. Each record must include:

- `contract_id`
- `function`
- `caller`
- `success`
- `timestamp_unix`

An `id` field is optional. When omitted, the detector derives a stable key from the contract, function, caller, and timestamp.
