#!/bin/bash

#======================================================================
# Validation Logic Test Suite
#======================================================================
#
# Error-handling policy (#1929):
#   * The script runs with `set -u -o pipefail` but deliberately NOT a global
#     `set -e`. This is a test runner: assert_fails / assert_success run
#     commands that are *expected* to exit non-zero, record the result in
#     FAILED and carry on, so one run reports every failure. A global
#     `set -e` would abort at the first expected failure.
#   * The setup section below DOES run under `set -e`. If the mock directory
#     or a mock tool cannot be created there is nothing valid to test, so stop
#     immediately instead of running the suite against a broken environment.

set -uo pipefail

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
NC='\033[0m'

# ── Setup (fail fast) ──────────────────────────────────────────────────
set -e

# Run from the repository root: the assertions below use relative script
# paths, and from any other directory they would fail with a misleading
# "Error mismatch" rather than saying the scripts were not found.
cd "$(dirname "${BASH_SOURCE[0]}")/.."
for required in scripts/deploy-soroban-testnet.sh scripts/validate-runtime-guards.sh; do
    if [ ! -f "$required" ]; then
        echo "ERROR: $required not found (working directory: $(pwd))" >&2
        exit 2
    fi
done

# Mock tools
MOCK_DIR=$(mktemp -d)
trap 'rm -rf "$MOCK_DIR"' EXIT

cat > "$MOCK_DIR/cargo" << 'EOF'
#!/bin/bash
exit 0
EOF
cat > "$MOCK_DIR/soroban" << 'EOF'
#!/bin/bash
exit 0
EOF
cat > "$MOCK_DIR/jq" << 'EOF'
#!/bin/bash
exit 0
EOF
cat > "$MOCK_DIR/curl" << 'EOF'
#!/bin/bash
exit 0
EOF
chmod +x "$MOCK_DIR"/*
export PATH="$MOCK_DIR:$PATH"

FAILED=0

# The assertions below intentionally run commands that fail.
set +e

assert_fails() {
    local cmd=$1
    local expected_error=$2
    local description=$3

    echo -n "Testing: $description... "
    
    # Run command and capture stderr
    output=$(eval "$cmd" 2>&1)
    exit_code=$?

    if [ $exit_code -eq 0 ]; then
        echo -e "${RED}FAILED${NC} (Expected failure, but got success)"
        FAILED=$((FAILED + 1))
        return 1
    fi

    # Herestring + -F: no pipe for pipefail/SIGPIPE to misreport on large output,
    # and the expected text is matched literally rather than as a regex.
    if grep -qF -- "$expected_error" <<<"$output"; then
        echo -e "${GREEN}PASSED${NC}"
    else
        echo -e "${RED}FAILED${NC} (Error mismatch)"
        echo "  Expected to find: $expected_error"
        echo "  Actual output: $output"
        FAILED=$((FAILED + 1))
        return 1
    fi
}

assert_success() {
    local cmd=$1
    local description=$2

    echo -n "Testing: $description... "
    
    output=$(eval "$cmd" 2>&1)
    exit_code=$?

    if [ $exit_code -eq 0 ]; then
        echo -e "${GREEN}PASSED${NC}"
    else
        echo -e "${RED}FAILED${NC} (Expected success, but got failure with exit code $exit_code)"
        echo "  Output: $output"
        FAILED=$((FAILED + 1))
        return 1
    fi
}

echo "Starting Validation Logic Tests..."
echo "----------------------------------"

# Test deploy-soroban-testnet.sh
# ------------------------------

# 1. Invalid Secret Key
export SOROBAN_SECRET_KEY="invalid"
assert_fails "bash ./scripts/deploy-soroban-testnet.sh --network testnet" "Invalid SOROBAN_SECRET_KEY format" "deploy: malformed secret key"

# 2. Missing Secret Key
unset SOROBAN_SECRET_KEY
assert_fails "bash ./scripts/deploy-soroban-testnet.sh --network testnet" "SOROBAN_SECRET_KEY environment variable not set" "deploy: missing secret key"

# 3. Invalid Network
export SOROBAN_SECRET_KEY="SAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"
assert_fails "bash ./scripts/deploy-soroban-testnet.sh --network invalid_net" "Invalid network: invalid_net" "deploy: invalid network"

# 4. Invalid Interval
assert_fails "bash ./scripts/deploy-soroban-testnet.sh --network testnet --interval -10" "Invalid validation interval: -10" "deploy: negative interval"
assert_fails "bash ./scripts/deploy-soroban-testnet.sh --network testnet --interval abc" "Invalid validation interval: abc" "deploy: non-numeric interval"

# Test validate-runtime-guards.sh
# -------------------------------

# 5. Missing Contract ID
assert_fails "bash ./scripts/validate-runtime-guards.sh" "Contract ID is required" "validate: missing contract id"

# 6. Invalid Contract ID
assert_fails "bash ./scripts/validate-runtime-guards.sh --contract-id invalid" "Invalid Contract ID format: invalid" "validate: malformed contract id"

# 7. Invalid Secret Key (if provided)
export SOROBAN_SECRET_KEY="invalid"
assert_fails "bash ./scripts/validate-runtime-guards.sh --contract-id CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA" "Invalid SOROBAN_SECRET_KEY format" "validate: malformed secret key"

# Summary
# -------
echo "----------------------------------"
if [ $FAILED -eq 0 ]; then
    echo -e "${GREEN}ALL TESTS PASSED${NC}"
    exit 0
else
    echo -e "${RED}$FAILED TESTS FAILED${NC}"
    exit 1
fi
