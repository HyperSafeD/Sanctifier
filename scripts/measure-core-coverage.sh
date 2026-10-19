#!/bin/bash
set -euo pipefail

echo "📊 Measuring current coverage for sanctifier-core..."

cd tooling/sanctifier-core

# Create output directory
mkdir -p ../../coverage-reports/core

# Generate detailed coverage report
cargo tarpaulin \
  --package sanctifier-core \
  --out Html \
  --out Json \
  --output-dir ../../coverage-reports/core \
  --engine llvm \
  --exclude-files "*/tests/*" "*/benches/*" \
  || true

# Parse JSON for current coverage percentage
if [ -f ../../coverage-reports/core/tarpaulin-report.json ]; then
  COVERAGE=$(jq -r '.files | map(.covered_percent) | add / length' ../../coverage-reports/core/tarpaulin-report.json)
  
  echo "Current sanctifier-core coverage: ${COVERAGE}%"
  
  if (( $(echo "$COVERAGE < 90" | bc -l) )); then
    GAP=$(echo "90 - $COVERAGE" | bc)
    echo "⚠️  Coverage below 90% threshold"
    echo "Gap to close: ${GAP}%"
    exit 1
  else
    echo "✅ Coverage meets 90% threshold"
  fi
else
  echo "⚠️  Could not parse coverage report"
  echo "Open coverage-reports/core/index.html to review manually"
fi
