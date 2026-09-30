#!/usr/bin/env bash
#
# Run all load test scenarios and generate combined report
#
# Usage: ./run-all-tests.sh [staging|production|localhost]
#

set -euo pipefail

ENV="${1:-staging}"
TIMESTAMP=$(date +%Y%m%d-%H%M%S)
RESULTS_DIR="results/${TIMESTAMP}"

# Color output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo "========================================"
echo "Sanctifier Load Test Suite"
echo "========================================"
echo "Environment: $ENV"
echo "Timestamp: $TIMESTAMP"
echo ""

# Set BASE_URL based on environment
case $ENV in
  staging)
    BASE_URL="https://staging.sanctifier.io"
    ;;
  production)
    BASE_URL="https://sanctifier.io"
    echo -e "${RED}WARNING: Running against PRODUCTION${NC}"
    read -p "Are you sure? (yes/no): " confirm
    if [ "$confirm" != "yes" ]; then
      echo "Aborted."
      exit 1
    fi
    ;;
  localhost)
    BASE_URL="http://localhost:3000"
    ;;
  *)
    echo -e "${RED}Invalid environment: $ENV${NC}"
    echo "Usage: $0 [staging|production|localhost]"
    exit 1
    ;;
esac

# Check if k6 is installed
if ! command -v k6 &> /dev/null; then
  echo -e "${RED}Error: k6 is not installed${NC}"
  echo "Install from: https://k6.io/docs/get-started/installation/"
  exit 1
fi

# Create results directory
mkdir -p "$RESULTS_DIR"

echo "Target URL: $BASE_URL"
echo ""

# Function to run a test scenario
run_test() {
  local scenario=$1
  local description=$2
  
  echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
  echo "Running: $scenario - $description"
  echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
  
  k6 run \
    --env BASE_URL="$BASE_URL" \
    --env SCENARIO="$scenario" \
    --out json="$RESULTS_DIR/${scenario}-output.json" \
    api-scan-load-test.js \
    | tee "$RESULTS_DIR/${scenario}-console.log"
  
  local exit_code=${PIPESTATUS[0]}
  
  if [ $exit_code -eq 0 ]; then
    echo -e "${GREEN}✅ $scenario PASSED${NC}"
  else
    echo -e "${RED}❌ $scenario FAILED (exit code: $exit_code)${NC}"
  fi
  
  echo ""
  return $exit_code
}

# Track overall success
OVERALL_SUCCESS=true

# Run smoke test
if run_test "smoke" "Quick sanity check"; then
  echo -e "${GREEN}Smoke test passed, proceeding with load tests${NC}"
else
  echo -e "${RED}Smoke test failed, aborting remaining tests${NC}"
  OVERALL_SUCCESS=false
  exit 1
fi

# Run load test
if ! run_test "load" "Normal mainnet traffic"; then
  OVERALL_SUCCESS=false
fi

# Run stress test
if ! run_test "stress" "3x normal load"; then
  OVERALL_SUCCESS=false
fi

# Run spike test
if ! run_test "spike" "Sudden traffic surge"; then
  OVERALL_SUCCESS=false
fi

# Generate combined report
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "Test Suite Complete"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

if [ "$OVERALL_SUCCESS" = true ]; then
  echo -e "${GREEN}✅ All tests PASSED${NC}"
  echo ""
  echo "Next steps:"
  echo "  1. Review detailed results in: $RESULTS_DIR/"
  echo "  2. Open HTML reports in browser"
  echo "  3. Document findings in mainnet signoff issue"
  exit 0
else
  echo -e "${RED}❌ Some tests FAILED${NC}"
  echo ""
  echo "Action required:"
  echo "  1. Review failed test logs in: $RESULTS_DIR/"
  echo "  2. Identify root cause (see frontend/tests/load/README.md)"
  echo "  3. Apply fixes and re-run tests"
  echo "  4. DO NOT proceed to mainnet until all tests pass"
  exit 1
fi
