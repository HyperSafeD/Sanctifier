#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import time
from dataclasses import asdict, dataclass
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


@dataclass(frozen=True)
class JsonPerfMetrics:
    path: str
    bytes: int
    read_ms: float
    parse_ms: float
    vulnerabilities: int | None = None
    max_pattern_len: int | None = None
    total_pattern_len: int | None = None


def load_json_metrics(path: Path) -> JsonPerfMetrics:
    start_read = time.perf_counter()
    raw = path.read_text(encoding="utf-8")
    end_read = time.perf_counter()

    start_parse = time.perf_counter()
    payload = json.loads(raw)
    end_parse = time.perf_counter()

    vulnerabilities = None
    max_pattern_len = None
    total_pattern_len = None
    if isinstance(payload, dict) and isinstance(payload.get("vulnerabilities"), list):
        vulns = payload.get("vulnerabilities") or []
        vulnerabilities = len(vulns)
        patterns = [str(v.get("pattern", "")) for v in vulns if isinstance(v, dict)]
        if patterns:
            max_pattern_len = max(len(p) for p in patterns)
            total_pattern_len = sum(len(p) for p in patterns)

    return JsonPerfMetrics(
        path=str(path.relative_to(ROOT)).replace("\\", "/"),
        bytes=path.stat().st_size,
        read_ms=(end_read - start_read) * 1000.0,
        parse_ms=(end_parse - start_parse) * 1000.0,
        vulnerabilities=vulnerabilities,
        max_pattern_len=max_pattern_len,
        total_pattern_len=total_pattern_len,
    )


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", default="", help="Optional JSON output file path")
    args = parser.parse_args()

    targets = [
        ROOT / "data" / "vulnerability-db.json",
        ROOT / "tooling" / "sanctifier-cli" / "data" / "vulnerability-db.json",
        ROOT / "schemas" / "vulnerability-db.json",
    ]

    report = {
        "generated_at_epoch_ms": int(time.time() * 1000),
        "metrics": [asdict(load_json_metrics(p)) for p in targets if p.exists()],
    }

    out = json.dumps(report, indent=2, sort_keys=True)
    if args.output:
        Path(args.output).write_text(out + "\n", encoding="utf-8")
    else:
        print(out)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

