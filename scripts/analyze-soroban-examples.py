#!/usr/bin/env python3
import json
import pathlib
import subprocess
import sys


def iter_contract_dirs(root: pathlib.Path):
    for lib_rs in sorted(root.rglob("src/lib.rs")):
        yield lib_rs.parent.parent


def analyze_contract(cli_path: pathlib.Path, contract_dir: pathlib.Path, root: pathlib.Path):
    proc = subprocess.run(
        [str(cli_path), "analyze", str(contract_dir), "--format", "json"],
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
        check=False,
    )
    report = json.loads(proc.stdout)
    summary = report.get("summary", {})
    return {
        "path": str(contract_dir.relative_to(root)).replace("\\", "/"),
        "exit_code": proc.returncode,
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
    aggregate = {
        "crates_scanned": len(results),
        "crates_with_findings": sum(1 for item in results if item["total_findings"] > 0),
        "crates_without_findings": sum(1 for item in results if item["total_findings"] == 0),
        "total_findings": sum(item["total_findings"] for item in results),
        "total_auth_gaps": sum(item["auth_gaps"] for item in results),
        "total_panic_issues": sum(item["panic_issues"] for item in results),
        "total_arithmetic_issues": sum(item["arithmetic_issues"] for item in results),
        "total_unhandled_results": sum(item["unhandled_results"] for item in results),
        "total_smt_issues": sum(item["smt_issues"] for item in results),
    }

    payload = {"aggregate": aggregate, "results": results}
    output_path.write_text(json.dumps(payload, indent=2), encoding="utf-8")
    print(json.dumps(aggregate, indent=2))


if __name__ == "__main__":
    main()
