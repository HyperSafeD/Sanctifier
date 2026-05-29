#!/usr/bin/env python3
from __future__ import annotations

import argparse
import re
import sys


_SEMVER_TAG_RE = re.compile(r"^v(?P<ver>\d+\.\d+\.\d+)$")
_CRATES_VERSION_RE = re.compile(r"^(v)?(?P<ver>\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?)$")


def resolve_version(*, action_ref: str, requested: str) -> str:
    """
    Returns a crates.io version string (e.g. "0.2.3") or "" if we should install latest.
    """
    req = (requested or "").strip()
    if req:
        # Accept both "0.2.3" and "v0.2.3" for convenience, but reject
        # arbitrary cargo-install arguments before they reach the shell.
        m = _CRATES_VERSION_RE.match(req)
        if not m:
            raise ValueError(
                "version must be a crates.io semantic version such as 0.2.3 or v0.2.3"
            )
        return m.group("ver")

    ref = (action_ref or "").strip()
    m = _SEMVER_TAG_RE.match(ref)
    if m:
        return m.group("ver")

    return ""


def main() -> int:
    p = argparse.ArgumentParser()
    p.add_argument("--action-ref", default="")
    p.add_argument("--requested", default="")
    p.add_argument("--output", required=True)
    args = p.parse_args()

    try:
        version = resolve_version(action_ref=args.action_ref, requested=args.requested)
    except ValueError as exc:
        print(f"Sanctifier action version error: {exc}", file=sys.stderr)
        return 2
    with open(args.output, "w", encoding="utf-8") as f:
        f.write(version)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

