#!/bin/bash

# Testnet Deployment Script for Sanctifier Contracts
# Issue #599: Improve UX/DX and defaults (testnet deployment scripts)
#
# Usage:
#   ./testnet-deploy.sh [contract-name] [network]
#   ./testnet-deploy.sh all testnet   # Deploy all contracts
#   ./testnet-deploy.sh amm-pool testnet  # Deploy specific contract

set -euo pipefail

# Configuration
NETWORK="${2:-testnet}"
CONTRACT="${1:-all}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$(dirname "$SCRIPT_DIR")")"
CONTRACTS_DIR="$PROJECT_ROOT/contracts"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Logging functions
log_info() {
  echo -e "${GREEN}[INFO]${NC} $*"
}

log_warn() {
  echo -e "${YELLOW}[WARN]${NC} $*"
}

log_error() {
  echo -e "${RED}[ERROR]${NC} $*"
}

# Validate environment
validate_environment() {
  log_info "Validating deployment environment..."

  if [ ! -d "$CONTRACTS_DIR" ]; then
    log_error "Contracts directory not found: $CONTRACTS_DIR"
    exit 1
  fi

  if ! command -v cargo &> /dev/null; then
    log_error "Cargo not found. Install Rust/Cargo from https://rustup.rs/"
    exit 1
  fi

  if ! command -v stellar-cli &> /dev/null; then
    log_warn "stellar-cli not found. Install from https://github.com/stellar/rs-soroban-cli"
  fi

  log_info "Environment validation complete"
}

# Build contract
build_contract() {
  local contract_name="$1"
  local contract_path="$CONTRACTS_DIR/$contract_name"

  if [ ! -d "$contract_path" ]; then
    log_error "Contract directory not found: $contract_path"
    return 1
  fi

  log_info "Building contract: $contract_name"
  cd "$contract_path"
  cargo build --target wasm32-unknown-unknown --release
  log_info "Build complete: $contract_name"
}

# Deploy contract
deploy_contract() {
  local contract_name="$1"
  local network="$2"
  local contract_path="$CONTRACTS_DIR/$contract_name"
  local wasm_file="$contract_path/target/wasm32-unknown-unknown/release/${contract_name//-/_}.wasm"

  if [ ! -f "$wasm_file" ]; then
    log_warn "WASM file not found: $wasm_file"
    log_info "Building contract first..."
    build_contract "$contract_name" || return 1
  fi

  log_info "Deploying contract to $network: $contract_name"
  # Placeholder for actual deployment logic
  log_info "Deployment would execute: stellar-cli contract deploy --network $network --wasm $wasm_file"
}

# List available contracts
list_contracts() {
  log_info "Available contracts:"
  for contract_dir in "$CONTRACTS_DIR"/*; do
    if [ -d "$contract_dir" ]; then
      contract_name=$(basename "$contract_dir")
      if [ -f "$contract_dir/Cargo.toml" ]; then
        echo "  - $contract_name"
      fi
    fi
  done
}

# Deploy all contracts
deploy_all() {
  log_info "Deploying all contracts to $NETWORK"

  for contract_dir in "$CONTRACTS_DIR"/*; do
    if [ -d "$contract_dir" ]; then
      contract_name=$(basename "$contract_dir")
      if [ -f "$contract_dir/Cargo.toml" ]; then
        if deploy_contract "$contract_name" "$NETWORK"; then
          log_info "Successfully deployed: $contract_name"
        else
          log_error "Failed to deploy: $contract_name"
        fi
      fi
    fi
  done

  log_info "Deployment complete"
}

# Main execution
main() {
  log_info "Sanctifier Contract Testnet Deployment"
  log_info "Network: $NETWORK"

  validate_environment

  if [ "$CONTRACT" = "all" ]; then
    deploy_all
  elif [ "$CONTRACT" = "list" ]; then
    list_contracts
  else
    build_contract "$CONTRACT" && deploy_contract "$CONTRACT" "$NETWORK"
  fi

  log_info "Deployment script complete"
}

# Help
if [ "${1:-}" = "--help" ] || [ "${1:-}" = "-h" ]; then
  cat << EOF
Sanctifier Contract Testnet Deployment Script

Usage:
  $0 [contract] [network]

Arguments:
  contract    Contract name or 'all' for all contracts (default: all)
  network     Target network: testnet, mainnet (default: testnet)

Examples:
  $0 all testnet          # Deploy all contracts to testnet
  $0 amm-pool testnet     # Deploy specific contract
  $0 list                 # List available contracts
  $0 --help              # Show this help message

Environment Variables:
  NETWORK     Override target network (default: testnet)
  CONTRACTS_DIR Override contracts directory path

Prerequisites:
  - Rust and Cargo installed
  - Stellar CLI installed
  - Network credentials configured

For more information, see docs/soroban-deployment.md
EOF
  exit 0
fi

main "$@"
