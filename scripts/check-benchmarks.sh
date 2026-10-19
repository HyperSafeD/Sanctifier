#!/bin/bash
# scripts/check-benchmarks.sh - Run all benchmarks and provide a summary.

set -e

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}Running SMT Latency Benchmarks...${NC}"
RUN_BENCHMARKS=1 cargo test -p sanctifier-core --test smt_latency_benchmark -- --nocapture

echo -e "\n${BLUE}Running Criterion AST Analysis Benchmarks...${NC}"
cargo bench -p sanctifier-core

echo -e "\n${GREEN}All benchmarks completed successfully!${NC}"
echo -e "SMT report: target/smt-latency-report.json"
echo -e "Criterion report: target/criterion/report/index.html"
