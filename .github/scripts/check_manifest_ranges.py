#!/usr/bin/env python3
"""Reject unbounded/floating dependency ranges in committed manifests.

Repositories that pin lockfiles must never declare a dependency so loosely
that the resolved set can drift out of sync with the committed lockfile.
This script scans the Cargo.toml manifests that have committed lockfiles and
every package.json with a committed package-lock.json, and fails when:

* a Cargo dependency version spec is unbounded (``*``, ``>=``, ``>``) or
  a Cargo git dependency does not pin an exact ``rev``;
* an npm dependency version is ``*``, ``latest``, an unbounded operator
  (``>=``/``>``) or an empty string.

Bounded caret/tilde ranges (``^1.2``, ``~0.1``) are allowed: they are
compatible with committed lockfiles and are kept in sync by the CI lockfile
checks, which refuse to run against stale locks.
"""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]


def _is_unbounded(spec: str) -> bool:
    return (
        spec in ("*", "latest", "") or spec.startswith(">") or spec.startswith("||")
    )


def _scan_package_json(path: Path, violations: list[str]) -> None:
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except (json.JSONDecodeError, OSError) as exc:
        violations.append(f"{path}: unreadable package.json: {exc}")
        return
    for section in (
        "dependencies",
        "devDependencies",
        "optionalDependencies",
        "peerDependencies",
        "overrides",
        "resolutions",
    ):
        deps = data.get(section) or {}
        for name, spec in deps.items():
            if _is_unbounded(str(spec)):
                violations.append(f"{path}: {section}.{name} = {spec!r}")


_CARGO_VERSION_RE = re.compile(r"""version\s*=\s*["']([^"']*)["']""")
_CARGO_GIT_ONLY_RE = re.compile(r"""git\s*=\s*["'][^"']+["']""")
_CARGO_REV_RE = re.compile(r"""rev\s*=\s*["'][^"']+["']""")


def _scan_cargo_toml(path: Path, violations: list[str]) -> None:
    text = path.read_text(encoding="utf-8")
    for line_no, line in enumerate(text.splitlines(), start=1):
        for match in _CARGO_VERSION_RE.finditer(line):
            spec = match.group(1)
            if spec in ("*", "") or spec.startswith(">") or spec.startswith("<*"):
                violations.append(f"{path}:{line_no}: version = {spec!r}")
        if _CARGO_GIT_ONLY_RE.search(line) and not _CARGO_REV_RE.search(line):
            violations.append(
                f"{path}:{line_no}: git dependency without pinned `rev`"
            )


def main() -> int:
    violations: list[str] = []

    cargo_manifests = [
        ROOT / "Cargo.toml",
        ROOT / "tooling" / "sanctifier-core" / "Cargo.toml",
        ROOT / "tooling" / "sanctifier-cli" / "Cargo.toml",
        ROOT / "tooling" / "sanctifier-wasm" / "Cargo.toml",
        ROOT / "tooling" / "sanctifier-detector" / "Cargo.toml",
    ]
    for manifest in cargo_manifests:
        if manifest.exists():
            _scan_cargo_toml(manifest, violations)

    npm_manifests = [
        ROOT / "package.json",
        ROOT / "frontend" / "package.json",
        ROOT / "vscode-extension" / "package.json",
    ]
    for manifest in npm_manifests:
        if manifest.exists():
            _scan_package_json(manifest, violations)

    if violations:
        print("Unbounded/floating dependency ranges detected:")
        for violation in violations:
            print(f"  - {violation}")
        print(
            "Pin these to a bounded range (e.g. ^1.2, ~0.1) and update the "
            "committed lockfile; the CI lockfile checks run with --locked."
        )
        return 1

    print("All committed manifests use bounded, lockfile-pinnable dependency ranges.")
    return 0


if __name__ == "__main__":
    sys.exit(main())