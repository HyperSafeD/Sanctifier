#!/bin/bash
set -euo pipefail

echo "📊 Updating benchmark baseline..."

if [ ! -f "benchmarks/current.json" ]; then
  echo "❌ Error: No current benchmark results found"
  echo "Run benchmarks first: cd tooling/sanctifier-core && cargo bench"
  exit 1
fi

cp benchmarks/current.json benchmarks/baseline.json

echo "✅ Baseline updated successfully"
echo ""
echo "Current baseline:"
jq '.benchmarks[] | "\(.name): \(.mean.estimate / 1000000) ms"' benchmarks/baseline.json | head -10

echo ""
echo "Don't forget to commit the updated baseline:"
echo "  git add benchmarks/baseline.json"
echo "  git commit -m 'chore: Update benchmark baseline'"
