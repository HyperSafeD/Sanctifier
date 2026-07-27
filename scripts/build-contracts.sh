#!/usr/bin/env bash
# build-contracts.sh — deterministic WASM build for all Sanctifier contracts.
#
# Goals
# -----
# 1. Reproducible output: SOURCE_DATE_EPOCH and RUSTFLAGS ensure that two
#    identical source trees produce byte-for-byte identical WASM binaries.
# 2. Performance budgets: each contract's WASM size is checked against the
#    limits in contracts/benchmark/src/budgets.rs; the build fails if any
#    contract exceeds its budget.
# 3. Toolchain lock: the script enforces the rust-version declared in
#    Cargo.toml (currently 1.78) so CI and local builds stay in sync.
#
# Usage
# -----
#   ./scripts/build-contracts.sh          # build all wasm-compatible contracts
#   ./scripts/build-contracts.sh --check  # build + size-budget check (CI mode)
#   CONTRACTS="my-contract timelock" ./scripts/build-contracts.sh  # subset
#
# Requirements
# ------------
#   - Rust toolchain with wasm32-unknown-unknown target installed
#   - cargo (comes with Rust)

set -euo pipefail

# ---------------------------------------------------------------------------
# Configuration
# ---------------------------------------------------------------------------

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT_DIR="${REPO_ROOT}/target/wasm32-unknown-unknown/release"
CHECK_MODE=false

# Contracts that can be compiled to wasm32-unknown-unknown (testutils only in
# [dev-dependencies]; see contracts/benchmark/src/budgets.rs for size limits).
WASM_CONTRACTS=(
    "flashloan-token"
    "governance-contract"
    "kani-poc-contract"
    "uups-proxy"
    "reentrancy-guard"
    "timelock"
    "token-with-bugs"
    "vulnerable-contract"
)

# WASM size budgets (bytes) — must stay in sync with budgets.rs.
declare -A WASM_BUDGET
WASM_BUDGET["flashloan-token"]=32768
WASM_BUDGET["governance-contract"]=65536
WASM_BUDGET["kani-poc-contract"]=16384
WASM_BUDGET["uups-proxy"]=32768
WASM_BUDGET["reentrancy-guard"]=16384
WASM_BUDGET["timelock"]=32768
WASM_BUDGET["token-with-bugs"]=32768
WASM_BUDGET["vulnerable-contract"]=32768

# ---------------------------------------------------------------------------
# Parse arguments
# ---------------------------------------------------------------------------

for arg in "$@"; do
    case "$arg" in
        --check) CHECK_MODE=true ;;
        *) echo "Unknown argument: $arg" >&2; exit 1 ;;
    esac
done

if [[ -n "${CONTRACTS:-}" ]]; then
    read -ra WASM_CONTRACTS <<< "$CONTRACTS"
fi

# ---------------------------------------------------------------------------
# Reproducibility: fix the timestamp embedded in WASM metadata.
# Uses the timestamp of the most recent git commit (or a fixed epoch if
# not inside a git repo).
# ---------------------------------------------------------------------------

if git -C "$REPO_ROOT" rev-parse --git-dir >/dev/null 2>&1; then
    SOURCE_DATE_EPOCH="$(git -C "$REPO_ROOT" log -1 --format=%ct)"
else
    SOURCE_DATE_EPOCH="0"
fi
export SOURCE_DATE_EPOCH

# Force codegen flags that are known to affect WASM output determinism.
export RUSTFLAGS="${RUSTFLAGS:-} -C opt-level=z -C lto=fat -C codegen-units=1"

# ---------------------------------------------------------------------------
# Toolchain version check
# ---------------------------------------------------------------------------

REQUIRED_RUST="1.78"
ACTUAL_RUST="$(rustc --version | awk '{print $2}')"

version_ge() {
    printf '%s\n%s\n' "$2" "$1" | sort -C -V
}

if ! version_ge "$ACTUAL_RUST" "$REQUIRED_RUST"; then
    echo "ERROR: Rust >= $REQUIRED_RUST required, found $ACTUAL_RUST" >&2
    exit 1
fi

echo "Rust ${ACTUAL_RUST} — SOURCE_DATE_EPOCH=${SOURCE_DATE_EPOCH}"

# ---------------------------------------------------------------------------
# Build loop
# ---------------------------------------------------------------------------

FAIL=0

for contract in "${WASM_CONTRACTS[@]}"; do
    echo ""
    echo "==> Building ${contract} ..."
    cargo build \
        -p "$contract" \
        --target wasm32-unknown-unknown \
        --release \
        --manifest-path "${REPO_ROOT}/Cargo.toml" 2>&1

    # Locate the WASM binary (crate name uses underscores in the file name).
    wasm_name="${contract//-/_}.wasm"
    wasm_path="${OUT_DIR}/${wasm_name}"

    if [[ ! -f "$wasm_path" ]]; then
        echo "ERROR: expected WASM not found at ${wasm_path}" >&2
        FAIL=1
        continue
    fi

    actual_bytes="$(wc -c < "$wasm_path")"
    budget_bytes="${WASM_BUDGET[$contract]:-0}"

    echo "    size: ${actual_bytes} bytes  (budget: ${budget_bytes} bytes)"

    if [[ "$CHECK_MODE" == true && "$budget_bytes" -gt 0 ]]; then
        if [[ "$actual_bytes" -gt "$budget_bytes" ]]; then
            echo "ERROR: ${contract} WASM (${actual_bytes}B) exceeds budget (${budget_bytes}B)" >&2
            FAIL=1
        else
            echo "    OK — within budget"
        fi
    fi
done

echo ""

if [[ "$FAIL" -ne 0 ]]; then
    echo "Build finished with errors." >&2
    exit 1
fi

echo "All contracts built successfully."
[[ "$CHECK_MODE" == true ]] && echo "All WASM size budgets satisfied."
