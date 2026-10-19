# Sanctifier Telegram Bot

A small Telegram bot for surfacing Sanctifier runtime-guard events with severity and contract-address filters.

## What it does

- `/latest` shows the newest events
- `/severity HIGH` filters by severity
- `/contract <address>` filters by contract address
- `/watch HIGH <address>` combines both filters

## Quickstart

1. Create a Telegram bot with [@BotFather](https://t.me/BotFather).
2. Export your token:

```bash
export TELEGRAM_BOT_TOKEN="123456:abc..."
```

3. Optionally point the bot at a local event cache:

```bash
export SANCTIFIER_EVENTS_FILE="./events.json"
```

4. Install dependencies and run the bot:

```bash
cd integrations/telegram
python3 -m pip install -r requirements.txt
python3 bot.py
```

## Event format

The bot expects a JSON array of objects with these fields:

```json
[
  {
    "id": "evt-001",
    "severity": "HIGH",
    "contract_address": "C...",
    "summary": "Human readable event summary",
    "timestamp": "2026-05-29T10:00:00Z"
  }
]
```

If `SANCTIFIER_EVENTS_FILE` is missing, the bot uses the built-in sample events so you can validate the command flow immediately.
