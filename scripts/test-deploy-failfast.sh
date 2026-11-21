#!/bin/bash
# Fail-fast regression tests for scripts/deploy.sh (#1923).
#
# Runs deploy.sh against stub `soroban`, `cargo`, `jq` and `sha256sum`
# binaries (no network, no keys), and checks both that it fails fast and that
# it still says *why*. Run from anywhere:
#
#   bash scripts/test-deploy-failfast.sh
set -u

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT
# Run a copy so deploy.sh's PROJECT_ROOT is the temp dir: a developer's own
# .env.local (which deploy.sh sources) can't leak a key into these tests.
mkdir -p "$WORK/scripts"
cp "$SCRIPT_DIR/deploy.sh" "$WORK/scripts/deploy.sh"
DEPLOY="$WORK/scripts/deploy.sh"
STUBS="$WORK/bin"
mkdir -p "$STUBS"
echo "wasm" > "$WORK/contract.wasm"

# soroban stub: behaviour is chosen per test with STUB_DEPLOY / STUB_SIMULATE.
cat > "$STUBS/soroban" << 'EOF'
#!/bin/bash
case "$*" in
  "network info"*) exit 0 ;;
  "contract deploy"*)
    case "${STUB_DEPLOY:-ok}" in
      ok)        echo "CABCDEFGHIJKLMNOPQRSTUVWXYZ234567ABCDEFGHIJKLMNOPQRSTUV2"; exit 0 ;;
      fail)      echo "error: tx failed: insufficient balance for fees"; exit 1 ;;
      weird)     echo "warning: something unexpected"; exit 0 ;;
    esac ;;
  "contract simulate"*)
    case "${STUB_SIMULATE:-ok}" in
      ok)        echo "simulated ok"; exit 0 ;;
      big)       seq 1 4000 | sed 's/^/simulation line /;s/$/ yyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyy/'; exit 0 ;;
      fail)      echo "error: simulation rejected"; exit 1 ;;
    esac ;;
esac
exit 0
EOF
printf '#!/bin/bash\nexit 0\n' > "$STUBS/cargo"
printf '#!/bin/bash\nexit 0\n' > "$STUBS/jq"
# deploy.sh's dry-run uses sha256sum, which macOS does not ship.
printf '#!/bin/bash\necho "deadbeef  $1"\n' > "$STUBS/sha256sum"
chmod +x "$STUBS"/*

KEY="SABCDEFGHIJKLMNOPQRSTUVWXYZ234567ABCDEFGHIJKLMNOPQRSTUV"
PASS=0
FAIL=0

# run <env assignments...> -- <deploy.sh args...>   -> sets OUT and CODE
run() {
    local envs=()
    while [[ $# -gt 0 && "$1" != "--" ]]; do envs+=("$1"); shift; done
    shift
    # ${envs[@]+...} keeps an empty array from tripping `set -u` on bash 3.2 (macOS);
    # -u scrubs any real key from the caller's environment.
    OUT="$(cd "$WORK" && env -u SOROBAN_SECRET_KEY PATH="$STUBS:$PATH" ${envs[@]+"${envs[@]}"} bash "$DEPLOY" "$@" 2>&1)"
    CODE=$?
}

check() { # check <description> <condition-exit-status>
    if [[ "$2" -eq 0 ]]; then
        echo "  ok   - $1"; PASS=$((PASS + 1))
    else
        echo "  FAIL - $1"; FAIL=$((FAIL + 1))
        echo "         exit code: $CODE"; echo "$OUT" | tail -8 | sed 's/^/         | /'
    fi
}
has() { grep -qF -- "$1" <<<"$OUT"; }

echo "deploy.sh fail-fast tests"

# 1. The header the issue asked for.
head -5 "$DEPLOY" | grep -q '^set -euo pipefail$'
check "sets 'set -euo pipefail' near the top" $?

# 2. Early validation exits non-zero with a reason.
run -- --network testnet --wasm "$WORK/contract.wasm"
[[ $CODE -ne 0 ]] && has "SOROBAN_SECRET_KEY not set"
check "missing SOROBAN_SECRET_KEY fails with a message" $?

run SOROBAN_SECRET_KEY="$KEY" -- --network mainnet --wasm "$WORK/contract.wasm"
[[ $CODE -ne 0 ]] && has "Mainnet requires --confirm-mainnet"
check "mainnet without --confirm-mainnet fails with a message" $?

run SOROBAN_SECRET_KEY="$KEY" -- --network testnet
[[ $CODE -ne 0 ]] && has "No WASM file found"
check "no WASM available fails with a message" $?

run SOROBAN_SECRET_KEY="$KEY" -- --bogus
[[ $CODE -ne 0 ]] && has "Unknown: --bogus"
check "unknown flag fails with a message" $?

# 3. Live deploy.
run SOROBAN_SECRET_KEY="$KEY" STUB_DEPLOY=ok -- --network testnet --wasm "$WORK/contract.wasm"
[[ $CODE -eq 0 ]] && has "Contract deployed: CABCDEFGHIJKLMNOPQRSTUVWXYZ234567ABCDEFGHIJKLMNOPQRSTUV2"
check "successful deploy exits 0 and reports the contract id" $?

run SOROBAN_SECRET_KEY="$KEY" STUB_DEPLOY=fail -- --network testnet --wasm "$WORK/contract.wasm"
[[ $CODE -ne 0 ]] && has "insufficient balance for fees"
check "failed deploy exits non-zero AND shows the CLI's error (not swallowed by set -e)" $?

run SOROBAN_SECRET_KEY="$KEY" STUB_DEPLOY=weird -- --network testnet --wasm "$WORK/contract.wasm"
[[ $CODE -ne 0 ]] && has "Deploy failed or unexpected output" && has "something unexpected"
check "unexpected deploy output exits non-zero and is shown" $?

# 4. Dry run.
run SOROBAN_SECRET_KEY="$KEY" STUB_SIMULATE=ok -- --network testnet --dry-run --wasm "$WORK/contract.wasm"
[[ $CODE -eq 0 ]] && has "Dry-run PASSED"
check "dry run with a passing simulation exits 0" $?

run SOROBAN_SECRET_KEY="$KEY" STUB_SIMULATE=big -- --network testnet --dry-run --wasm "$WORK/contract.wasm"
[[ $CODE -eq 0 ]] && has "Dry-run PASSED" && has "simulation line 1 "
check "dry run with very large simulate output still passes (no SIGPIPE/pipefail exit)" $?

run SOROBAN_SECRET_KEY="$KEY" STUB_SIMULATE=fail -- --network testnet --dry-run --wasm "$WORK/contract.wasm"
[[ $CODE -ne 0 ]] && has "Dry-run FAILED" && has "simulation rejected"
check "dry run with a failing simulation exits non-zero and shows why" $?

echo
echo "$PASS passed, $FAIL failed"
[[ $FAIL -eq 0 ]]
