#!/bin/bash
set -euo pipefail

DURATION_HOURS=${1:-24}
FUZZ_TARGET=${2:-"all"}

echo "🔍 Starting extended fuzz campaign"
echo "Duration: ${DURATION_HOURS} hours"
echo "Target: ${FUZZ_TARGET}"

cd tooling/sanctifier-core/fuzz

# Create campaign output directory
CAMPAIGN_DIR="campaigns/$(date +%Y%m%d-%H%M%S)"
mkdir -p "$CAMPAIGN_DIR"

# Log system information
echo "System Information:" > "$CAMPAIGN_DIR/system-info.txt"
uname -a >> "$CAMPAIGN_DIR/system-info.txt"
rustc --version >> "$CAMPAIGN_DIR/system-info.txt"
cargo fuzz --version >> "$CAMPAIGN_DIR/system-info.txt" || echo "cargo-fuzz not installed" >> "$CAMPAIGN_DIR/system-info.txt"

# Build fuzz targets
echo "Building fuzz targets..."
cargo fuzz build

# List available targets
TARGETS=$(cargo fuzz list)
echo "Available fuzz targets:"
echo "$TARGETS"

if [ "$FUZZ_TARGET" = "all" ]; then
  FUZZ_TARGETS=$TARGETS
else
  FUZZ_TARGETS=$FUZZ_TARGET
fi

# Calculate seconds for duration
DURATION_SECONDS=$((DURATION_HOURS * 3600))

echo "Starting campaign at $(date)"
echo "$DURATION_SECONDS" > "$CAMPAIGN_DIR/planned-duration.txt"
date +%s > "$CAMPAIGN_DIR/start-time.txt"

# Run each target (sequentially for simplicity, can be parallelized)
for target in $FUZZ_TARGETS; do
  echo "Fuzzing target: $target"
  
  cargo fuzz run "$target" \
    --jobs $(nproc) \
    --release \
    -- \
    -max_total_time="$DURATION_SECONDS" \
    -print_final_stats=1 \
    -artifact_prefix="$CAMPAIGN_DIR/" \
    2>&1 | tee "$CAMPAIGN_DIR/${target}.log" &
  
  # Store PID for monitoring
  echo $! >> "$CAMPAIGN_DIR/pids.txt"
done

echo ""
echo "Campaign running in background."
echo "Monitor with: tail -f $CAMPAIGN_DIR/*.log"
echo "Stop with: kill \$(cat $CAMPAIGN_DIR/pids.txt)"
echo ""
echo "Campaign directory: $CAMPAIGN_DIR"
