#!/usr/bin/env python3
import json
import os
import pathlib
import subprocess
import sys

# Per-example wall-clock budget in seconds.  Overridable via the
# SANCTIFIER_EXAMPLE_TIMEOUT environment variable for slower CI machines.
_DEFAULT_TIMEOUT = 60
EXAMPLE_TIMEOUT: int = int(os.environ.get("SANCTIFIER_EXAMPLE_TIMEOUT", _DEFAULT_TIMEOUT))


def iter_contract_dirs(root: pathlib.Path):
    for lib_rs in sorted(root.rglob("src/lib.rs")):
        yield lib_rs.parent.parent


def analyze_contract(cli_path: pathlib.Path, contract_dir: pathlib.Path, root: pathlib.Path):
    rel_path = str(contract_dir.relative_to(root)).replace("\\", "/")
    try:
        proc = subprocess.run(
            [str(cli_path), "analyze", str(contract_dir), "--format", "json"],
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            check=False,
            timeout=EXAMPLE_TIMEOUT,
        )
    except subprocess.TimeoutExpired:
        print(
            f"[TIMEOUT] {rel_path} did not complete within {EXAMPLE_TIMEOUT}s — skipping",
            file=sys.stderr,
        )
        return {
            "path": rel_path,
            "exit_code": -1,
            "timed_out": True,
            "total_findings": 0,
            "auth_gaps": 0,
            "panic_issues": 0,
            "arithmetic_issues": 0,
            "size_warnings": 0,
            "unhandled_results": 0,
            "smt_issues": 0,
        }

    report = json.loads(proc.stdout) if proc.stdout.strip() else {}
    summary = report.get("summary", {})
    return {
        "path": rel_path,
        "exit_code": proc.returncode,
        "timed_out": False,
        "total_findings": summary.get("total_findings", 0),
        "auth_gaps": summary.get("auth_gaps", 0),
        "panic_issues": summary.get("panic_issues", 0),
        "arithmetic_issues": summary.get("arithmetic_issues", 0),
        "size_warnings": summary.get("size_warnings", 0),
        "unhandled_results": summary.get("unhandled_results", 0),
        "smt_issues": summary.get("smt_issues", 0),
    }


def main():
    if len(sys.argv) != 4:
        raise SystemExit(
            "usage: analyze-soroban-examples.py <examples-root> <sanctifier-cli> <output-json>"
        )

    root = pathlib.Path(sys.argv[1]).resolve()
    cli_path = pathlib.Path(sys.argv[2]).resolve()
    output_path = pathlib.Path(sys.argv[3]).resolve()

    results = [analyze_contract(cli_path, contract_dir, root) for contract_dir in iter_contract_dirs(root)]
    timed_out = [r for r in results if r.get("timed_out")]
    aggregate = {
        "crates_scanned": len(results),
        "crates_timed_out": len(timed_out),
        "crates_with_findings": sum(1 for item in results if item["total_findings"] > 0),
        "crates_without_findings": sum(1 for item in results if item["total_findings"] == 0 and not item.get("timed_out")),
        "total_findings": sum(item["total_findings"] for item in results),
        "total_auth_gaps": sum(item["auth_gaps"] for item in results),
        "total_panic_issues": sum(item["panic_issues"] for item in results),
        "total_arithmetic_issues": sum(item["arithmetic_issues"] for item in results),
        "total_unhandled_results": sum(item["unhandled_results"] for item in results),
        "total_smt_issues": sum(item["smt_issues"] for item in results),
    }
    if timed_out:
        print(
            f"[WARNING] {len(timed_out)} example(s) timed out after {EXAMPLE_TIMEOUT}s: "
            + ", ".join(r["path"] for r in timed_out),
            file=sys.stderr,
        )

    payload = {"aggregate": aggregate, "results": results}
    output_path.write_text(json.dumps(payload, indent=2), encoding="utf-8")
    print(json.dumps(aggregate, indent=2))


if __name__ == "__main__":
    main()
