#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
from pathlib import Path


def count_sarif_findings(sarif_path: Path) -> int:
    try:
        data = json.loads(sarif_path.read_text(encoding="utf-8"))
    except FileNotFoundError:
        return 0
    except json.JSONDecodeError as exc:
        raise ValueError(f"invalid SARIF JSON in {sarif_path}: {exc}") from exc

    return sum(len(run.get("results", []) or []) for run in (data.get("runs", []) or []))


def count_json_findings(payload: str) -> int:
    try:
        data = json.loads(payload or "{}")
    except json.JSONDecodeError as exc:
        raise ValueError(f"invalid Sanctifier JSON output: {exc}") from exc

    summary = data.get("summary") or {}
    total = summary.get("total_findings")
    if total is not None:
        return int(total)
    return sum(int(summary.get(key, 0) or 0) for key in ("critical", "high", "medium", "low", "info"))


def summarize_findings(*, fmt: str, path: str, min_severity: str, sarif_output: str) -> int:
    if fmt == "sarif":
        return count_sarif_findings(Path(sarif_output))
    if fmt != "json":
        return 0

    proc = subprocess.run(
        ["sanctifier", "analyze", path, "--format", "json", "--min-severity", min_severity],
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
        text=True,
        check=False,
    )
    return count_json_findings(proc.stdout)


def append_github_output(name: str, value: int) -> None:
    output = os.environ.get("GITHUB_OUTPUT")
    if not output:
        return
    with open(output, "a", encoding="utf-8") as handle:
        handle.write(f"{name}={value}\n")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--format", default=os.environ.get("INPUT_FORMAT", "sarif"))
    parser.add_argument("--path", default=os.environ.get("INPUT_PATH", "."))
    parser.add_argument("--min-severity", default=os.environ.get("INPUT_MIN_SEVERITY", "high"))
    parser.add_argument("--sarif-output", default=os.environ.get("INPUT_SARIF_OUTPUT", "sanctifier-results.sarif"))
    args = parser.parse_args()

    try:
        findings_count = summarize_findings(
            fmt=(args.format or "").strip().lower(),
            path=args.path,
            min_severity=(args.min_severity or "").strip().lower(),
            sarif_output=args.sarif_output,
        )
    except ValueError as exc:
        print(f"Sanctifier action summary error: {exc}", file=sys.stderr)
        return 2

    append_github_output("findings-count", findings_count)
    print(f"findings-count={findings_count}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
