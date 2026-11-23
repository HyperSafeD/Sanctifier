#!/usr/bin/env python3
"""
Benchmark regression threshold check for CI.

Compares a current benchmark snapshot against a stored baseline and exits
non-zero if any metric regresses beyond the configured threshold.

Metrics checked
---------------
From data_schemas_perf.json:
  - kb_per_sec   : source KB processed per second  (derived from read_ms + bytes)
  - rules_per_sec: vulnerability rules applied per second (parse_ms + vulnerabilities)

From docs_nav_perf.json:
  - dead_link_rate: dead_links_count / max(total_links, 1)  (must not grow)

Regression definition
---------------------
A metric *regresses* when its current value is worse than the baseline by more
than THRESHOLD (default 20 %).  "Worse" means:
  - lower  for throughput metrics (kb_per_sec, rules_per_sec)
  - higher for error-rate metrics (dead_link_rate)

Baseline management
-------------------
  --update-baseline   Write the current snapshot as the new baseline and exit 0.
                      Run this on the main branch after a deliberate perf change.

Usage
-----
  # Check only (CI default):
  python3 benchmarks/check_regression.py \
      --data   benchmarks/data_schemas_perf.json \
      --docs   benchmarks/docs_nav_perf.json \
      --baseline benchmarks/baseline.json

  # Update baseline (run locally / post-merge on main):
  python3 benchmarks/check_regression.py ... --update-baseline
"""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import NamedTuple

DEFAULT_THRESHOLD = 0.20  # 20 %


class Metric(NamedTuple):
    name: str
    current: float
    baseline: float
    threshold: float
    higher_is_better: bool

    @property
    def ratio(self) -> float:
        """current / baseline, or 0 if baseline is 0."""
        return self.current / self.baseline if self.baseline else 0.0

    @property
    def regressed(self) -> bool:
        if self.baseline == 0.0:
            return False
        if self.higher_is_better:
            # regressed when current dropped more than threshold below baseline
            return self.ratio < (1.0 - self.threshold)
        else:
            # regressed when current rose more than threshold above baseline
            return self.ratio > (1.0 + self.threshold)

    def summary(self) -> str:
        direction = "▼" if self.higher_is_better else "▲"
        change_pct = (self.ratio - 1.0) * 100.0
        status = "FAIL" if self.regressed else "ok  "
        return (
            f"[{status}] {self.name:<20} "
            f"baseline={self.baseline:.4f}  current={self.current:.4f}  "
            f"change={change_pct:+.1f}%  threshold={self.threshold*100:.0f}%"
            + (f"  {direction} REGRESSION" if self.regressed else "")
        )


def _derive_data_metrics(data: dict) -> dict[str, float]:
    metrics = data.get("metrics", [])
    total_bytes: float = sum(m.get("bytes", 0) for m in metrics)
    total_read_ms: float = sum(m.get("read_ms", 0.0) for m in metrics)
    total_parse_ms: float = sum(m.get("parse_ms", 0.0) for m in metrics)
    total_rules: float = sum(
        (m.get("vulnerabilities") or 0) for m in metrics
    )

    kb_per_sec = (
        (total_bytes / 1024.0) / (total_read_ms / 1000.0)
        if total_read_ms > 0
        else 0.0
    )
    rules_per_sec = (
        total_rules / (total_parse_ms / 1000.0) if total_parse_ms > 0 else 0.0
    )
    return {"kb_per_sec": kb_per_sec, "rules_per_sec": rules_per_sec}


def _derive_docs_metrics(docs: dict) -> dict[str, float]:
    total = max(docs.get("total_links", 0), 1)
    dead = docs.get("dead_links_count", 0)
    return {"dead_link_rate": dead / total}


def load_json(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def build_metrics(
    data_snap: dict,
    docs_snap: dict,
    baseline: dict,
    threshold: float,
) -> list[Metric]:
    current_data = _derive_data_metrics(data_snap)
    current_docs = _derive_docs_metrics(docs_snap)

    baseline_data = baseline.get("data_metrics", {})
    baseline_docs = baseline.get("docs_metrics", {})

    return [
        Metric(
            name="kb_per_sec",
            current=current_data["kb_per_sec"],
            baseline=baseline_data.get("kb_per_sec", 0.0),
            threshold=threshold,
            higher_is_better=True,
        ),
        Metric(
            name="rules_per_sec",
            current=current_data["rules_per_sec"],
            baseline=baseline_docs.get("rules_per_sec",
                                        baseline_data.get("rules_per_sec", 0.0)),
            threshold=threshold,
            higher_is_better=True,
        ),
        Metric(
            name="dead_link_rate",
            current=current_docs["dead_link_rate"],
            baseline=baseline_docs.get("dead_link_rate", 0.0),
            threshold=threshold,
            higher_is_better=False,
        ),
    ]


def make_baseline_payload(data_snap: dict, docs_snap: dict) -> dict:
    return {
        "version": "1.0",
        "note": "Generated by check_regression.py --update-baseline",
        "data_metrics": _derive_data_metrics(data_snap),
        "docs_metrics": _derive_docs_metrics(docs_snap),
        "benchmarks": [],
    }


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--data",
        default="benchmarks/data_schemas_perf.json",
        help="Path to data_schemas_perf.json snapshot",
    )
    parser.add_argument(
        "--docs",
        default="benchmarks/docs_nav_perf.json",
        help="Path to docs_nav_perf.json snapshot",
    )
    parser.add_argument(
        "--baseline",
        default="benchmarks/baseline.json",
        help="Path to stored baseline JSON",
    )
    parser.add_argument(
        "--threshold",
        type=float,
        default=DEFAULT_THRESHOLD,
        help="Regression threshold as a fraction (default 0.20 = 20%%)",
    )
    parser.add_argument(
        "--update-baseline",
        action="store_true",
        help="Overwrite the baseline with current snapshot values and exit 0",
    )
    args = parser.parse_args(argv)

    data_path = Path(args.data)
    docs_path = Path(args.docs)
    baseline_path = Path(args.baseline)

    if not data_path.exists():
        print(f"ERROR: data snapshot not found: {data_path}", file=sys.stderr)
        return 1
    if not docs_path.exists():
        print(f"ERROR: docs snapshot not found: {docs_path}", file=sys.stderr)
        return 1

    data_snap = load_json(data_path)
    docs_snap = load_json(docs_path)

    if args.update_baseline:
        payload = make_baseline_payload(data_snap, docs_snap)
        baseline_path.write_text(json.dumps(payload, indent=2), encoding="utf-8")
        print(f"Baseline updated: {baseline_path}")
        return 0

    if not baseline_path.exists():
        print(
            f"WARNING: baseline not found at {baseline_path} — skipping regression check.\n"
            "Run with --update-baseline on the main branch to seed it.",
            file=sys.stderr,
        )
        return 0

    baseline = load_json(baseline_path)

    # Baseline seeded with empty benchmarks list (placeholder) — skip check.
    has_data = baseline.get("data_metrics") or baseline.get("docs_metrics")
    if not has_data:
        print(
            "WARNING: baseline contains no metrics yet — skipping regression check.\n"
            "Run with --update-baseline to seed it.",
            file=sys.stderr,
        )
        return 0

    metrics = build_metrics(data_snap, docs_snap, baseline, args.threshold)

    print("Benchmark regression check")
    print(f"  threshold : {args.threshold * 100:.0f}%")
    print(f"  baseline  : {baseline_path}")
    print()
    regressions = []
    for m in metrics:
        print(" ", m.summary())
        if m.regressed:
            regressions.append(m)

    print()
    if regressions:
        print(
            f"FAILED: {len(regressions)} regression(s) detected "
            f"(>{args.threshold * 100:.0f}% worse than baseline):"
        )
        for m in regressions:
            print(f"  - {m.name}")
        return 1

    print("All metrics within threshold.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
