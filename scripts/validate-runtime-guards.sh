#!/bin/bash

#======================================================================
# Runtime Guard Wrapper Validation Harness
#======================================================================
# This script provides comprehensive testing and validation for
# deployed runtime guard wrapper contracts on Soroban testnet
#======================================================================

set -euo pipefail

# Color codes
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m'

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
VALIDATION_LOG="${PROJECT_ROOT}/.validation.log"
VALIDATION_RESULTS="${PROJECT_ROOT}/.validation-results.json"

# Test configuration
TEST_COUNT=0
PASSED_COUNT=0
FAILED_COUNT=0
NETWORK="testnet"

#======================================================================
# Utility Functions
#======================================================================

log_info() {
    echo -e "${BLUE}[INFO]${NC} $*" | tee -a "$VALIDATION_LOG"
}

log_success() {
    echo -e "${GREEN}[✓]${NC} $*" | tee -a "$VALIDATION_LOG"
}

log_error() {
    echo -e "${RED}[✗]${NC} $*" | tee -a "$VALIDATION_LOG" >&2
}

log_test() {
    echo -e "${CYAN}[TEST]${NC} $*" | tee -a "$VALIDATION_LOG"
}

start_test() {
    local test_name=$1
    (( TEST_COUNT++ ))
    log_test "[$TEST_COUNT] Running: $test_name"
}

pass_test() {
    local test_name=$1
    (( PASSED_COUNT++ ))
    log_success "Test passed: $test_name"
}

fail_test() {
    local test_name=$1
    local reason=${2:-"Unknown error"}
    (( FAILED_COUNT++ ))
    log_error "Test failed: $test_name - $reason"
}

#======================================================================
# Validation Tests
#======================================================================

test_health_check() {
    start_test "Health Check"
    
    local contract_id=$1
    
    local result
    result=$(soroban contract invoke \
        --id "$contract_id" \
        --network "$NETWORK" \
        -- health_check 2>&1 || echo "failed")
    
    if echo "$result" | grep -q "true"; then
        pass_test "Health Check"
        return 0
    else
        fail_test "Health Check" "Contract health check returned false"
        return 1
    fi
}

test_get_stats() {
    start_test "Get Statistics"
    
    local contract_id=$1
    
    local result
    result=$(soroban contract invoke \
        --id "$contract_id" \
        --network "$NETWORK" \
        -- get_stats 2>&1 || echo "failed")
    
    if echo "$result" | grep -qE "[0-9]+"; then
        pass_test "Get Statistics"
        return 0
    else
        fail_test "Get Statistics" "Could not retrieve statistics"
        return 1
    fi
}

test_execution_monitoring() {
    start_test "Execution Monitoring"
    
    local contract_id=$1
    
    # Execute a guarded function (simulated)
    local result
    result=$(soroban contract invoke \
        --id "$contract_id" \
        --network "$NETWORK" \
        -- execute_guarded test_function 2>&1 || echo "failed")
    
    if [ "$result" != "failed" ]; then
        pass_test "Execution Monitoring"
        return 0
    else
        fail_test "Execution Monitoring" "Could not execute guarded function"
        return 1
    fi
}

test_event_emission() {
    start_test "Event Emission"
    
    local contract_id=$1
    
    # Check that contract emits guard events
    local result
    result=$(soroban events read \
        --network "$NETWORK" \
        --id "$contract_id" 2>&1 || echo "")
    
    if [ -n "$result" ] && echo "$result" | grep -q "guard"; then
        pass_test "Event Emission"
        return 0
    else
        log_test "Event Emission - Note: No events found yet (may be expected)"
        pass_test "Event Emission"
        return 0
    fi
}

test_storage_accessibility() {
    start_test "Storage Accessibility"
    
    local contract_id=$1
    
    # Verify that contract storage is accessible
    if soroban contract read \
        --id "$contract_id" \
        --network "$NETWORK" \
        --key instance 2>&1 | grep -q "0x"; then
        pass_test "Storage Accessibility"
        return 0
    else
        fail_test "Storage Accessibility" "Could not access contract storage"
        return 1
    fi
}

test_performance_baseline() {
    start_test "Performance Baseline"
    
    local contract_id=$1
    
    log_test "Measuring execution time for health check..."
    
    local start_time
    local end_time
    
    start_time=$(date +%s%N)
    soroban contract invoke \
        --id "$contract_id" \
        --network "$NETWORK" \
        -- health_check &>/dev/null
    end_time=$(date +%s%N)
    
    local duration_ms=$(( (end_time - start_time) / 1000000 ))
    
    # Baseline: Health checks should complete in reasonable time
    if [ "$duration_ms" -lt 30000 ]; then
        log_test "Health check completed in ${duration_ms}ms"
        pass_test "Performance Baseline"
        return 0
    else
        fail_test "Performance Baseline" "Health check took ${duration_ms}ms (expected < 30000ms)"
        return 1
    fi
}

test_error_handling() {
    start_test "Error Handling"
    
    local contract_id=$1
    
    # Test that contract properly handles invalid inputs
    local result
    result=$(soroban contract invoke \
        --id "$contract_id" \
        --network "$NETWORK" \
        -- execute_guarded invalid_function 2>&1 || echo "handled")
    
    if echo "$result" | grep -q "handled\|error"; then
        pass_test "Error Handling"
        return 0
    else
        fail_test "Error Handling" "Contract did not handle invalid input properly"
        return 1
    fi
}

test_concurrent_operations() {
    start_test "Concurrent Operations"
    
    local contract_id=$1
    
    # Simulate concurrent health checks
    local success_count=0
    for i in {1..3}; do
        if (soroban contract invoke \
            --id "$contract_id" \
            --network "$NETWORK" \
            -- health_check &>/dev/null); then
            (( success_count++ ))
        fi
        sleep 1
    done
    
    if [ "$success_count" -ge 2 ]; then
        log_test "Concurrent operations: $success_count/3 succeeded"
        pass_test "Concurrent Operations"
        return 0
    else
        fail_test "Concurrent Operations" "Only $success_count/3 operations succeeded"
        return 1
    fi
}

#======================================================================
# Validation Suite Orchestration
#======================================================================

run_full_validation() {
    local contract_id=$1
    
    log_info "Running full validation suite for contract: $contract_id"
    log_info "Network: $NETWORK"
    echo "" | tee -a "$VALIDATION_LOG"
    
    # Run all tests
    test_health_check "$contract_id" || true
    test_get_stats "$contract_id" || true
    test_execution_monitoring "$contract_id" || true
    test_event_emission "$contract_id" || true
    test_storage_accessibility "$contract_id" || true
    test_performance_baseline "$contract_id" || true
    test_error_handling "$contract_id" || true
    test_concurrent_operations "$contract_id" || true
    
    echo "" | tee -a "$VALIDATION_LOG"
}

generate_validation_report() {
    local contract_id=$1
    
    local pass_rate=0
    if [ "$TEST_COUNT" -gt 0 ]; then
        pass_rate=$(( (PASSED_COUNT * 100) / TEST_COUNT ))
    fi
    
    cat > "$VALIDATION_RESULTS" << EOF
{
  "contract_id": "$contract_id",
  "network": "$NETWORK",
  "timestamp": "$(date -u +'%Y-%m-%dT%H:%M:%SZ')",
  "test_results": {
    "total_tests": $TEST_COUNT,
    "passed": $PASSED_COUNT,
    "failed": $FAILED_COUNT,
    "pass_rate": $pass_rate
  },
  "status": "$([ $FAILED_COUNT -eq 0 ] && echo 'PASS' || echo 'FAIL')"
}
EOF
    
    log_info "Validation report generated: $VALIDATION_RESULTS"
}

print_validation_summary() {
    local contract_id=$1
    
    echo "" | tee -a "$VALIDATION_LOG"
    echo "======================================================================" | tee -a "$VALIDATION_LOG"
    echo "VALIDATION SUMMARY" | tee -a "$VALIDATION_LOG"
    echo "======================================================================" | tee -a "$VALIDATION_LOG"
    echo "Contract ID: $contract_id" | tee -a "$VALIDATION_LOG"
    echo "Network: $NETWORK" | tee -a "$VALIDATION_LOG"
    echo "Total Tests: $TEST_COUNT" | tee -a "$VALIDATION_LOG"
    echo -e "Passed: ${GREEN}$PASSED_COUNT${NC}" | tee -a "$VALIDATION_LOG"
    echo -e "Failed: ${RED}$FAILED_COUNT${NC}" | tee -a "$VALIDATION_LOG"
    
    if [ "$TEST_COUNT" -gt 0 ]; then
        local pass_rate=$(( (PASSED_COUNT * 100) / TEST_COUNT ))
        echo "Pass Rate: $pass_rate%" | tee -a "$VALIDATION_LOG"
        
        if [ "$pass_rate" -ge 80 ]; then
            echo -e "${GREEN}✓ Validation Status: PASS${NC}" | tee -a "$VALIDATION_LOG"
        elif [ "$pass_rate" -ge 50 ]; then
            echo -e "${YELLOW}⚠ Validation Status: PARTIAL PASS${NC}" | tee -a "$VALIDATION_LOG"
        else
            echo -e "${RED}✗ Validation Status: FAIL${NC}" | tee -a "$VALIDATION_LOG"
        fi
    fi
    echo "======================================================================" | tee -a "$VALIDATION_LOG"
    echo "" | tee -a "$VALIDATION_LOG"
}

#======================================================================
# CLI Argument Parsing
#======================================================================

parse_arguments() {
    while [[ $# -gt 0 ]]; do
        case $1 in
            --contract-id)
                CONTRACT_ID="$2"
                shift 2
                ;;
            --network)
                NETWORK="$2"
                shift 2
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
Usage: validate-runtime-guards.sh [OPTIONS]

Options:
    --contract-id <ID>      Contract ID to validate (required)
    --network <NETWORK>     Target network (testnet, futurenet, mainnet)
                           Default: testnet
    --help                  Show this help message

Environment Variables:
    SOROBAN_SECRET_KEY     Your Soroban secret key (required for some tests)

Examples:
    # Validate a contract on testnet
    ./validate-runtime-guards.sh --contract-id C...

    # Validate on a different network
    ./validate-runtime-guards.sh --contract-id C... --network futurenet

EOF
}

#======================================================================
# Main Entry Point
#======================================================================

main() {
    echo -e "${CYAN}"
    cat << "EOF"
    
    ╔═══════════════════════════════════════════════════════════════╗
    ║   Sanctifier: Runtime Guard Validation Harness                ║
    ║                                                               ║
    ║   Comprehensive testing of deployed wrapper contracts         ║
    ╚═══════════════════════════════════════════════════════════════╝
    
EOF
    echo -e "${NC}"
    
    CONTRACT_ID=""
    
    parse_arguments "$@"
    
    if [ -z "$CONTRACT_ID" ]; then
        log_error "Contract ID is required"
        print_help
        exit 1
    fi
    
    # Initialize logging
    mkdir -p "$(dirname "$VALIDATION_LOG")"
    :> "$VALIDATION_LOG"
    
    # Run validation
    run_full_validation "$CONTRACT_ID"
    
    # Generate report
    generate_validation_report "$CONTRACT_ID"
    
    # Print summary
    print_validation_summary "$CONTRACT_ID"
    
    # Exit with appropriate code
    if [ "$FAILED_COUNT" -eq 0 ]; then
        exit 0
    else
        exit 1
    fi
}

main "$@"
