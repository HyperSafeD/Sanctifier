#!/usr/bin/env python3
from __future__ import annotations

import argparse
import os
import re
import sys
from dataclasses import dataclass
from pathlib import Path


_ALLOWED_FORMATS = {"text", "json", "sarif"}
_ALLOWED_SEVERITIES = {"critical", "high", "medium", "low", "info"}
_SAFE_RELATIVE_PATH_RE = re.compile(r"^[A-Za-z0-9._/\-\\ ]+$")


@dataclass(frozen=True)
class ActionInputs:
    path: str
    min_severity: str
    format: str
    upload_sarif: str
    sarif_output: str
    debug: str


def _normalise_bool(value: str, *, name: str) -> str:
    lowered = (value or "").strip().lower()
    if lowered in {"true", "1", "yes"}:
        return "true"
    if lowered in {"false", "0", "no"}:
        return "false"
    raise ValueError(f"{name} must be true or false, got {value!r}")


def _validate_path(value: str, *, name: str, allow_missing: bool) -> str:
    raw = (value or "").strip()
    if not raw:
        raise ValueError(f"{name} must not be empty")
    if "\x00" in raw or "\n" in raw or "\r" in raw:
        raise ValueError(f"{name} must not contain control characters")
    if raw.startswith("-"):
        raise ValueError(f"{name} must not start with '-'")

    path = Path(raw).expanduser()
    if path.is_absolute():
        raise ValueError(f"{name} must be relative to the checked-out repository")
    if any(part == ".." for part in path.parts):
        raise ValueError(f"{name} must not contain '..' path traversal segments")
    if not _SAFE_RELATIVE_PATH_RE.match(raw):
        raise ValueError(f"{name} contains unsupported characters")
    if not allow_missing and not path.exists():
        raise ValueError(f"{name} does not exist in the checked-out repository: {raw}")
    return raw


def validate_inputs(
    *,
    path: str,
    min_severity: str,
    format: str,
    upload_sarif: str,
    sarif_output: str,
    debug: str,
) -> ActionInputs:
    fmt = (format or "").strip().lower()
    if fmt not in _ALLOWED_FORMATS:
        allowed = ", ".join(sorted(_ALLOWED_FORMATS))
        raise ValueError(f"format must be one of {allowed}, got {format!r}")

    severity = (min_severity or "").strip().lower()
    if severity not in _ALLOWED_SEVERITIES:
        allowed = ", ".join(sorted(_ALLOWED_SEVERITIES))
        raise ValueError(f"min-severity must be one of {allowed}, got {min_severity!r}")

    return ActionInputs(
        path=_validate_path(path, name="path", allow_missing=False),
        min_severity=severity,
        format=fmt,
        upload_sarif=_normalise_bool(upload_sarif, name="upload-sarif"),
        sarif_output=_validate_path(sarif_output, name="sarif-output", allow_missing=True),
        debug=_normalise_bool(debug, name="debug"),
    )


def write_env_file(inputs: ActionInputs, output: Path) -> None:
    output.write_text(
        "\n".join(
            [
                f"SANCTIFIER_ACTION_PATH={inputs.path}",
                f"SANCTIFIER_ACTION_MIN_SEVERITY={inputs.min_severity}",
                f"SANCTIFIER_ACTION_FORMAT={inputs.format}",
                f"SANCTIFIER_ACTION_UPLOAD_SARIF={inputs.upload_sarif}",
                f"SANCTIFIER_ACTION_SARIF_OUTPUT={inputs.sarif_output}",
                f"SANCTIFIER_ACTION_DEBUG={inputs.debug}",
                "",
            ]
        ),
        encoding="utf-8",
    )


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--path", default=os.environ.get("INPUT_PATH", "."))
    parser.add_argument("--min-severity", default=os.environ.get("INPUT_MIN_SEVERITY", "high"))
    parser.add_argument("--format", default=os.environ.get("INPUT_FORMAT", "sarif"))
    parser.add_argument("--upload-sarif", default=os.environ.get("INPUT_UPLOAD_SARIF", "true"))
    parser.add_argument("--sarif-output", default=os.environ.get("INPUT_SARIF_OUTPUT", "sanctifier-results.sarif"))
    parser.add_argument("--debug", default=os.environ.get("INPUT_DEBUG", "false"))
    parser.add_argument("--output", required=True)
    args = parser.parse_args()

    try:
        inputs = validate_inputs(
            path=args.path,
            min_severity=args.min_severity,
            format=args.format,
            upload_sarif=args.upload_sarif,
            sarif_output=args.sarif_output,
            debug=args.debug,
        )
    except ValueError as exc:
        print(f"::error title=Invalid Input::Sanctifier action input error: {exc}", file=sys.stderr)
        return 2

    if inputs.debug == "true":
        print(
            "[sanctifier-action][debug] "
            f"path={inputs.path!r} format={inputs.format!r} "
            f"min_severity={inputs.min_severity!r} upload_sarif={inputs.upload_sarif!r} "
            f"sarif_output={inputs.sarif_output!r}",
            file=sys.stderr,
        )

    write_env_file(inputs, Path(args.output))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
