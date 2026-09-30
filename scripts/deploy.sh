#!/bin/bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

# ── Colours ──────────────────────────────────────────────────────────────────────
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m'

log_info()    { echo -e "${BLUE}[INFO]${NC}  $*"; }
log_success() { echo -e "${GREEN}[OK]${NC}   $*"; }
log_warning() { echo -e "${YELLOW}[WARN]${NC} $*"; }
log_error()   { echo -e "${RED}[ERR]${NC}  $*" >&2; }

# ── Defaults ─────────────────────────────────────────────────────────────────────
NETWORK="testnet"
CONFIRM_MAINNET=false
DRY_RUN=false
WASM_PATH=""

# ── Help ─────────────────────────────────────────────────────────────────────────
print_help() {
    cat << 'HELP'
Usage: deploy.sh [OPTIONS]

Options:
    --network <NETWORK>         Target network (testnet, futurenet, mainnet)
                                Default: testnet
    --confirm-mainnet           Acknowledge mainnet target (required for mainnet)
    --dry-run                   Simulate deployment without broadcasting
    --wasm <PATH>               Path to WASM file (auto-detected if omitted)
    --help                      Show this help

Environment:
    SOROBAN_SECRET_KEY          Secret key for signing (required)
HELP
}

# ── Parse args ───────────────────────────────────────────────────────────────────
parse_args() {
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --network)         NETWORK="$2"; shift 2 ;;
            --confirm-mainnet) CONFIRM_MAINNET=true; shift ;;
            --dry-run)         DRY_RUN=true; shift ;;
            --wasm)            WASM_PATH="$2"; shift 2 ;;
            --help)            print_help; exit 0 ;;
            *)                 log_error "Unknown: $1"; print_help; exit 1 ;;
        esac
    done
}

# ── Validate environment ─────────────────────────────────────────────────────────
validate_env() {
    if [[ -f "${PROJECT_ROOT}/.env.local" ]]; then
        set -a; source "${PROJECT_ROOT}/.env.local"; set +a
    fi

    if [[ -z "${SOROBAN_SECRET_KEY:-}" ]]; then
        log_error "SOROBAN_SECRET_KEY not set (use .env.local or export)"
        exit 1
    fi

    for cmd in soroban cargo jq; do
        if ! command -v "$cmd" &>/dev/null; then
            log_error "Required tool missing: $cmd"
            exit 1
        fi
    done
}

# ── Network guard (#1124, #1133) ─────────────────────────────────────────────────
check_network() {
    log_info "Target network: $NETWORK"

    if [[ "$NETWORK" == "mainnet" ]]; then
        if [[ "$CONFIRM_MAINNET" != "true" ]]; then
            log_error "Mainnet requires --confirm-mainnet flag"
            exit 1
        fi
        log_warning "Mainnet deployment confirmed. Proceeding with caution."
    fi

    if ! soroban network info --network "$NETWORK" &>/dev/null; then
        log_error "Network not reachable: $NETWORK"
        exit 1
    fi
    log_success "Network reachable: $NETWORK"
}

# ── Resolve WASM path ────────────────────────────────────────────────────────────
resolve_wasm() {
    if [[ -n "$WASM_PATH" ]]; then
        if [[ ! -f "$WASM_PATH" ]]; then
            log_error "WASM not found: $WASM_PATH"
            exit 1
        fi
        echo "$WASM_PATH"
        return
    fi

    local default="target/wasm32-unknown-unknown/release/vulnerable_contract.wasm"
    if [[ -f "$default" ]]; then
        echo "$default"
        return
    fi

    local found
    found=$(find target/wasm32-unknown-unknown/release -name "*.wasm" 2>/dev/null | head -1)
    if [[ -n "$found" ]]; then
        echo "$found"
        return
    fi

    log_error "No WASM file found. Build first or pass --wasm."
    exit 1
}

# ── Preflight / simulation ───────────────────────────────────────────────────────
dry_run_deploy() {
    local wasm="$1"
    local wasm_hash
    wasm_hash=$(sha256sum "$wasm" | awk '{print $1}')

    echo ""
    echo "╔═══════════════════════════════════════════════════════════════╗"
    echo "║              DRY RUN — Transaction Summary                   ║"
    echo "╚═══════════════════════════════════════════════════════════════╝"
    echo ""
    echo "  Network:        $NETWORK"
    echo "  WASM file:      $wasm"
    echo "  WASM size:      $(du -h "$wasm" | cut -f1)"
    echo "  WASM SHA-256:   $wasm_hash"
    echo "  Signer:         ${SOROBAN_SECRET_KEY:0:6}...${SOROBAN_SECRET_KEY: -4}"
    echo ""

    # Preflight via simulate — this validates the deploy would succeed
    log_info "Running simulation (soroban contract simulate)..."
    local sim_output
    if sim_output=$(soroban contract simulate \
        --wasm "$wasm" \
        --source "$SOROBAN_SECRET_KEY" \
        --network "$NETWORK" 2>&1); then
        log_success "Simulation succeeded"
        echo ""
        echo "  ── Simulated transaction ──"
        echo "$sim_output" | head -20
        echo ""
        echo "  ✅ Dry-run PASSED — no broadcast made."
        echo ""
        return 0
    else
        log_error "Simulation FAILED:"
        echo "$sim_output"
        echo ""
        echo "  ❌ Dry-run FAILED — the deploy would not succeed."
        echo ""
        return 1
    fi
}

live_deploy() {
    local wasm="$1"
    log_info "Deploying contract to $NETWORK ..."

    local output
    output=$(soroban contract deploy \
        --wasm "$wasm" \
        --source "$SOROBAN_SECRET_KEY" \
        --network "$NETWORK" 2>&1)

    if echo "$output" | grep -qE "^[A-Z0-9]{56}$"; then
        local cid
        cid=$(echo "$output" | grep -oE "^[A-Z0-9]{56}$" | head -1)
        log_success "Contract deployed: $cid"
    else
        log_error "Deploy failed or unexpected output:"
        echo "$output"
        exit 1
    fi
}

# ── Main ─────────────────────────────────────────────────────────────────────────
main() {
    parse_args "$@"
    validate_env
    check_network

    echo ""
    echo "=============================================="
    echo "  Sanctifier — Contract Deployment"
    echo "  Network:    $NETWORK"
    echo "  Mode:       $( $DRY_RUN && echo 'DRY RUN (no broadcast)' || echo 'LIVE' )"
    echo "=============================================="
    echo ""

    local wasm
    wasm=$(resolve_wasm)

    if [[ "$DRY_RUN" == "true" ]]; then
        dry_run_deploy "$wasm"
    else
        live_deploy "$wasm"
    fi
}

main "$@"
