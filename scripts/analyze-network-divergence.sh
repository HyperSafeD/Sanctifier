#!/bin/bash
set -euo pipefail

echo "🔍 Analyzing cross-network test results..."

RESULTS_DIR="test-results"
DIVERGENCES_FOUND=false

for network in testnet futurenet mainnet-fork; do
  RESULT_FILE="$RESULTS_DIR/test-results-$network.json"
  
  if [ ! -f "$RESULT_FILE" ]; then
    echo "⚠️  Missing results for $network"
    continue
  fi
  
  # Simple failure count (adjust based on actual result format)
  FAILURES=$(jq -r '.failures // [] | length' "$RESULT_FILE" 2>/dev/null || echo "0")
  
  if [ "$FAILURES" -gt 0 ]; then
    echo "❌ $network: $FAILURES test(s) failed"
    DIVERGENCES_FOUND=true
    
    jq -r '.failures[]? | "  - \(.test_name // "unknown"): \(.error // "no error message")"' "$RESULT_FILE" 2>/dev/null || echo "  - Unable to parse failure details"
  else
    echo "✅ $network: All tests passed"
  fi
done

if [ "$DIVERGENCES_FOUND" = true ]; then
  echo ""
  echo "⚠️  Network divergences detected. Review failed tests and either:"
  echo "   1. Fix the divergence (preferred)"
  echo "   2. Document as expected behavior"
  echo "   3. Add network-specific test configuration"
  exit 1
fi

echo ""
echo "✅ No divergences found across all networks"
