#!/usr/bin/env python3
"""
Sanctifier Telegram Bot
Provides lightweight event lookup and alert filtering for runtime-guard activity.
"""

from __future__ import annotations

import json
import os
import time
from pathlib import Path
from typing import Any, Iterable

import requests

TELEGRAM_TOKEN = os.getenv("TELEGRAM_BOT_TOKEN")
EVENTS_FILE = os.getenv("SANCTIFIER_EVENTS_FILE", "events.json")
POLL_INTERVAL = int(os.getenv("SANCTIFIER_TELEGRAM_POLL_INTERVAL", "10"))


def load_events() -> list[dict[str, Any]]:
    path = Path(EVENTS_FILE)
    if not path.exists():
        return SAMPLE_EVENTS.copy()

    try:
        payload = json.loads(path.read_text(encoding="utf-8"))
        if isinstance(payload, list):
            return [item for item in payload if isinstance(item, dict)]
    except json.JSONDecodeError:
        pass
    return SAMPLE_EVENTS.copy()


def sample_events() -> list[dict[str, Any]]:
    return [
        {
            "id": "evt-001",
            "severity": "HIGH",
            "contract_address": "CBLDEREKXK6AIZ7ZSKC6VYCK4MKF4FZ4ANJEU67QZAQUG57I4KGZMTXB",
            "summary": "Runtime guard detected repeated call bursts on testnet.",
            "timestamp": "2026-05-29T10:00:00Z",
        },
        {
            "id": "evt-002",
            "severity": "MEDIUM",
            "contract_address": "CDDVM5A5IVDAG5FZ2OU2CLWAHC7A2T7LHQHZSDVKZPE6SDMDO2JCR3UY",
            "summary": "Validation run completed with a storage warning.",
            "timestamp": "2026-05-29T10:05:00Z",
        },
    ]


SAMPLE_EVENTS = sample_events()


def filter_events(
    events: Iterable[dict[str, Any]],
    severity: str | None = None,
    contract_address: str | None = None,
) -> list[dict[str, Any]]:
    severity = severity.upper() if severity else None
    contract_address = contract_address.upper() if contract_address else None

    filtered: list[dict[str, Any]] = []
    for event in events:
        event_severity = str(event.get("severity", "")).upper()
        event_contract = str(event.get("contract_address", "")).upper()
        if severity and event_severity != severity:
            continue
        if contract_address and event_contract != contract_address:
            continue
        filtered.append(event)
    return filtered


def format_event(event: dict[str, Any]) -> str:
    return (
        f"[{event.get('severity', 'UNKNOWN')}] {event.get('id', 'unknown')}\n"
        f"Contract: {event.get('contract_address', 'n/a')}\n"
        f"Time: {event.get('timestamp', 'n/a')}\n"
        f"{event.get('summary', '')}"
    )


def send_message(chat_id: int, text: str) -> None:
    url = f"https://api.telegram.org/bot{TELEGRAM_TOKEN}/sendMessage"
    requests.post(
        url,
        json={
            "chat_id": chat_id,
            "text": text,
            "disable_web_page_preview": True,
        },
        timeout=10,
    ).raise_for_status()


def handle_command(chat_id: int, text: str) -> None:
    tokens = text.strip().split()
    command = tokens[0].lower() if tokens else ""
    args = tokens[1:]
    events = load_events()

    if command == "/start":
        send_message(
            chat_id,
            "Sanctifier Telegram bot is ready.\n"
            "Commands: /latest, /severity <level>, /contract <address>",
        )
        return

    if command == "/latest":
        matches = events[:5]
    elif command == "/severity":
        if not args:
            send_message(chat_id, "Usage: /severity <LOW|MEDIUM|HIGH|CRITICAL>")
            return
        matches = filter_events(events, severity=args[0])
    elif command == "/contract":
        if not args:
            send_message(chat_id, "Usage: /contract <contract address>")
            return
        matches = filter_events(events, contract_address=args[0])
    elif command == "/watch":
        if not args:
            send_message(chat_id, "Usage: /watch <severity> [contract address]")
            return
        severity = args[0]
        contract = args[1] if len(args) > 1 else None
        matches = filter_events(events, severity=severity, contract_address=contract)
    else:
        send_message(
            chat_id,
            "Unknown command. Use /latest, /severity <level>, /contract <address>, or /watch <severity> [address].",
        )
        return

    if not matches:
        send_message(chat_id, "No matching events found.")
        return

    message = "\n\n".join(format_event(event) for event in matches[:5])
    send_message(chat_id, message)


def run() -> None:
    if not TELEGRAM_TOKEN:
        raise SystemExit("TELEGRAM_BOT_TOKEN is required")

    offset = 0
    while True:
        response = requests.get(
            f"https://api.telegram.org/bot{TELEGRAM_TOKEN}/getUpdates",
            params={"timeout": POLL_INTERVAL, "offset": offset},
            timeout=POLL_INTERVAL + 5,
        )
        response.raise_for_status()
        payload = response.json()
        for update in payload.get("result", []):
            offset = max(offset, int(update["update_id"]) + 1)
            message = update.get("message") or {}
            chat = message.get("chat") or {}
            text = message.get("text")
            if not text or "id" not in chat:
                continue
            if text.startswith("/"):
                handle_command(int(chat["id"]), text)
        time.sleep(1)


if __name__ == "__main__":
    run()
