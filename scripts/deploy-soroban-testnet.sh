#!/bin/bash

#======================================================================
# Soroban Runtime Guard Wrapper Deployment Script
#======================================================================
# This script automates:
# 1. Building runtime guard wrapper contracts
# 2. Deploying to Soroban testnet
# 3. Validating deployments with continuous health checks
# 4. Monitoring and logging all deployments
#======================================================================

set -euo pipefail

# Color codes for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
CONTRACTS_DIR="${PROJECT_ROOT}/contracts"
DEPLOYMENT_LOG="${PROJECT_ROOT}/.deployment.log"
DEPLOYMENT_MANIFEST="${PROJECT_ROOT}/.deployment-manifest.json"
TEMP_DIR=$(mktemp -d)

# Default values
NETWORK="testnet"
VALIDATE_AFTER_DEPLOY=true
CONTINUOUS_VALIDATION=true
VALIDATION_INTERVAL=300  # 5 minutes
MAX_RETRIES=3
DRY_RUN=false

#======================================================================
# Logging and Utility Functions
#======================================================================

log_info() {
    echo -e "${BLUE}[INFO]${NC} $*" | tee -a "$DEPLOYMENT_LOG"
}

log_success() {
    echo -e "${GREEN}[✓]${NC} $*" | tee -a "$DEPLOYMENT_LOG"
}

log_error() {
    echo -e "${RED}[✗]${NC} $*" | tee -a "$DEPLOYMENT_LOG" >&2
}

log_warning() {
    echo -e "${YELLOW}[⚠]${NC} $*" | tee -a "$DEPLOYMENT_LOG"
}

log_debug() {
    if [[ "${DEBUG:-false}" == "true" ]]; then
        echo -e "${CYAN}[DEBUG]${NC} $*" | tee -a "$DEPLOYMENT_LOG"
    fi
}

print_banner() {
    cat << "EOF"
    
    ╔═══════════════════════════════════════════════════════════════╗
    ║       Sanctifier: Soroban Runtime Guard Deployment            ║
    ║                                                               ║
    ║  Automating deployment of runtime guard wrapper contracts     ║
    ║  to Soroban testnet for continuous validation                 ║
    ╚═══════════════════════════════════════════════════════════════╝
    
EOF
}

cleanup() {
    rm -rf "$TEMP_DIR"
    log_debug "Cleaned up temporary files"
}

trap cleanup EXIT

#======================================================================
# Environment Validation
#======================================================================

validate_environment() {
    log_info "Validating environment..."

    # Check required tools
    local required_tools=("cargo" "soroban" "jq" "curl")
    for tool in "${required_tools[@]}"; do
        if ! command -v "$tool" &> /dev/null; then
            log_error "Required tool not found: $tool"
            return 1
        fi
    done

    # Check Soroban secret key
    if [[ -z "${SOROBAN_SECRET_KEY:-}" ]]; then
        log_error "SOROBAN_SECRET_KEY environment variable not set"
        return 1
    fi

    # Check network is valid
    case "$NETWORK" in
        testnet|futurenet|mainnet)
            log_success "Network validated: $NETWORK"
            ;;
        *)
            log_error "Invalid network: $NETWORK"
            return 1
            ;;
    esac

    return 0
}

#======================================================================
# Contract Building
#======================================================================

build_contract() {
    local contract_path=$1
    local contract_name=$(basename "$contract_path")

    log_info "Building contract: $contract_name"

    if [[ ! -f "$contract_path/Cargo.toml" ]]; then
        log_error "Cargo.toml not found in $contract_path"
        return 1
    fi

    if [[ "$DRY_RUN" == "false" ]]; then
        if ! cargo build \
            --manifest-path "$contract_path/Cargo.toml" \
            --release \
            --target wasm32-unknown-unknown 2>&1 | tee -a "$DEPLOYMENT_LOG"; then
            log_error "Failed to build contract: $contract_name"
            return 1
        fi
    fi

    log_success "Contract built: $contract_name"
    return 0
}

find_wasm_file() {
    local contract_path=$1
    local wasm_path="${contract_path}/target/wasm32-unknown-unknown/release"

    if [[ ! -d "$wasm_path" ]]; then
        log_error "WASM directory not found: $wasm_path"
        return 1
    fi

    # Find the first .wasm file
    local wasm_file
    wasm_file=$(find "$wasm_path" -maxdepth 1 -name "*.wasm" | head -1)

    if [[ -z "$wasm_file" ]]; then
        log_error "No WASM file found in $wasm_path"
        return 1
    fi

    echo "$wasm_file"
    return 0
}

#======================================================================
# Contract Deployment
#======================================================================

deploy_contract() {
    local wasm_file=$1
    local contract_name=$(basename "$wasm_file" .wasm)
    local retry_count=0

    log_info "Deploying contract: $contract_name to $NETWORK"

    while (( retry_count < MAX_RETRIES )); do
        if [[ "$DRY_RUN" == "false" ]]; then
            local deploy_output
            deploy_output=$(soroban contract deploy \
                --wasm "$wasm_file" \
                --source "$SOROBAN_SECRET_KEY" \
                --network "$NETWORK" 2>&1 || true)

            if echo "$deploy_output" | grep -qE "^[A-Z0-9]{56}$"; then
                local contract_id
                contract_id=$(echo "$deploy_output" | grep -oE "^[A-Z0-9]{56}$" | head -1)
                log_success "Contract deployed: $contract_id"
                echo "$contract_id"
                return 0
            else
                log_warning "Deployment attempt $((retry_count + 1))/$MAX_RETRIES failed"
                (( retry_count++ ))
                sleep 5
                continue
            fi
        else
            # Dry run: generate mock contract ID
            local mock_contract_id
            mock_contract_id="C$(openssl rand -hex 27 | tr a-f A-F)"
            log_success "[DRY RUN] Contract would deploy: $mock_contract_id"
            echo "$mock_contract_id"
            return 0
        fi
    done

    log_error "Failed to deploy contract after $MAX_RETRIES attempts"
    return 1
}

#======================================================================
# Contract Validation
#======================================================================

validate_contract() {
    local contract_id=$1
    local validation_name="health_check"

    log_info "Validating contract: $contract_id"

    if [[ "$DRY_RUN" == "false" ]]; then
        local validation_output
        validation_output=$(soroban contract invoke \
            --id "$contract_id" \
            --network "$NETWORK" \
            -- \
            "$validation_name" 2>&1 || true)

        if echo "$validation_output" | grep -q "true\|success"; then
            log_success "Contract validation passed: $contract_id"
            return 0
        else
            log_error "Contract validation failed: $contract_id"
            return 1
        fi
    else
        log_success "[DRY RUN] Contract validation would pass"
        return 0
    fi
}

#======================================================================
# Manifest Management
#======================================================================

initialize_manifest() {
    if [[ ! -f "$DEPLOYMENT_MANIFEST" ]]; then
        cat > "$DEPLOYMENT_MANIFEST" << 'EOF'
{
  "version": "1.0",
  "deployments": [],
  "last_updated": null,
  "validation_status": null
}
EOF
        log_debug "Initialized deployment manifest"
    fi
}

add_deployment_to_manifest() {
    local contract_id=$1
    local contract_name=$2
    local wasm_file=$3

    local wasm_hash
    wasm_hash=$(sha256sum "$wasm_file" | awk '{print $1}')

    local timestamp
    timestamp=$(date -u +"%Y-%m-%dT%H:%M:%SZ")

    local temp_manifest=$(mktemp)
    jq --arg id "$contract_id" \
       --arg name "$contract_name" \
       --arg hash "$wasm_hash" \
       --arg ts "$timestamp" \
       --arg net "$NETWORK" \
       '.deployments += [{
           "contract_id": $id,
           "name": $name,
           "wasm_hash": $hash,
           "network": $net,
           "deployed_at": $ts,
           "last_validated": $ts,
           "status": "active"
       }] | .last_updated = $ts' \
       "$DEPLOYMENT_MANIFEST" > "$temp_manifest"

    mv "$temp_manifest" "$DEPLOYMENT_MANIFEST"
    log_debug "Added deployment to manifest: $contract_id"
}

#======================================================================
# Continuous Validation
#======================================================================

continuous_validation_loop() {
    local contracts_json=$1
    local iteration=0

    log_info "Starting continuous validation loop (interval: ${VALIDATION_INTERVAL}s)"

    while true; do
        (( iteration++ ))
        log_info "Validation iteration #$iteration"

        echo "$contracts_json" | jq -r '.[] | .contract_id' | while read -r contract_id; do
            if ! validate_contract "$contract_id"; then
                log_error "Validation failed for contract: $contract_id"
                # Update manifest
                jq --arg id "$contract_id" \
                   '.deployments[] |= if .contract_id == $id then .status = "validation_failed" else . end' \
                   "$DEPLOYMENT_MANIFEST" > "${DEPLOYMENT_MANIFEST}.tmp"
                mv "${DEPLOYMENT_MANIFEST}.tmp" "$DEPLOYMENT_MANIFEST"
            fi
        done

        log_debug "Waiting ${VALIDATION_INTERVAL}s before next validation cycle..."
        sleep "$VALIDATION_INTERVAL"
    done
}

#======================================================================
# Main Deployment Orchestration
#======================================================================

deploy_runtime_guards() {
    log_info "Starting runtime guard wrapper deployment process"

    initialize_manifest

    # Find all contract directories
    local contracts_to_deploy=()
    if [[ -d "$CONTRACTS_DIR/runtime-guard-wrapper" ]]; then
        contracts_to_deploy+=("$CONTRACTS_DIR/runtime-guard-wrapper")
    fi

    if [[ ${#contracts_to_deploy[@]} -eq 0 ]]; then
        log_warning "No contracts found to deploy"
        return 0
    fi

    log_info "Found ${#contracts_to_deploy[@]} contract(s) to deploy"

    # Deploy each contract
    local deployments_json="[]"
    for contract_path in "${contracts_to_deploy[@]}"; do
        local contract_name=$(basename "$contract_path")

        log_info "Processing contract: $contract_name"

        # Build
        if ! build_contract "$contract_path"; then
            log_error "Skipping $contract_name due to build failure"
            continue
        fi

        # Find WASM
        local wasm_file
        if ! wasm_file=$(find_wasm_file "$contract_path"); then
            log_error "Skipping $contract_name due to missing WASM"
            continue
        fi

        # Deploy
        local contract_id
        if ! contract_id=$(deploy_contract "$wasm_file"); then
            log_error "Skipping $contract_name due to deployment failure"
            continue
        fi

        # Validate
        if $VALIDATE_AFTER_DEPLOY; then
            if ! validate_contract "$contract_id"; then
                log_warning "Post-deployment validation failed for $contract_id"
            fi
        fi

        # Add to manifest
        add_deployment_to_manifest "$contract_id" "$contract_name" "$wasm_file"

        # Update deployments JSON
        deployments_json=$(echo "$deployments_json" | jq --arg id "$contract_id" \
            --arg name "$contract_name" \
            '. += [{"contract_id": $id, "name": $name}]')

        log_success "Successfully deployed: $contract_name ($contract_id)"
    done

    log_info "Deployment phase completed"

    # Start continuous validation if enabled
    if $CONTINUOUS_VALIDATION && [[ $(echo "$deployments_json" | jq 'length') -gt 0 ]]; then
        continuous_validation_loop "$deployments_json"
    fi
}

#======================================================================
# CLI Argument Parsing
#======================================================================

parse_arguments() {
    while [[ $# -gt 0 ]]; do
        case $1 in
            --network)
                NETWORK="$2"
                shift 2
                ;;
            --no-validate)
                VALIDATE_AFTER_DEPLOY=false
                CONTINUOUS_VALIDATION=false
                shift
                ;;
            --no-continuous)
                CONTINUOUS_VALIDATION=false
                shift
                ;;
            --dry-run)
                DRY_RUN=true
                shift
                ;;
            --interval)
                VALIDATION_INTERVAL="$2"
                shift 2
                ;;
            --debug)
                DEBUG=true
                shift
                ;;
            --help)
                print_help
                exit 0
                ;;
            *)
                log_error "Unknown argument: $1"
                print_help
                exit 1
                ;;
        esac
    done
}

print_help() {
    cat << 'EOF'
Usage: deploy-soroban-testnet.sh [OPTIONS]

Options:
    --network <NETWORK>         Target network (testnet, futurenet, mainnet)
                               Default: testnet
    --no-validate              Skip validation after deployment
    --no-continuous            Disable continuous validation loop
    --dry-run                  Perform dry run without actual deployment
    --interval <SECONDS>       Validation interval in seconds
                              Default: 300
    --debug                    Enable debug logging
    --help                     Show this help message

Environment Variables:
    SOROBAN_SECRET_KEY        Your Soroban secret key (required)
    DEBUG                     Set to 'true' to enable debug logging

Examples:
    # Deploy to testnet with continuous validation
    ./deploy-soroban-testnet.sh --network testnet

    # Perform a dry run
    ./deploy-soroban-testnet.sh --dry-run

    # Deploy without continuous validation
    ./deploy-soroban-testnet.sh --no-continuous

EOF
}

#======================================================================
# Main Entry Point
#======================================================================

main() {
    print_banner

    parse_arguments "$@"

    # Initialize logging
    mkdir -p "$(dirname "$DEPLOYMENT_LOG")"
    :> "$DEPLOYMENT_LOG"

    log_info "Deployment script started"
    log_info "Network: $NETWORK"
    log_info "Validate after deploy: $VALIDATE_AFTER_DEPLOY"
    log_info "Continuous validation: $CONTINUOUS_VALIDATION"
    log_debug "Validation interval: ${VALIDATION_INTERVAL}s"

    if ! validate_environment; then
        log_error "Environment validation failed"
        exit 1
    fi

    deploy_runtime_guards

    log_success "Deployment process completed"
    log_info "Deployment manifest: $DEPLOYMENT_MANIFEST"
    log_info "Deployment log: $DEPLOYMENT_LOG"
}

main "$@"
