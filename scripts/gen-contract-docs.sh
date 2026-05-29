#!/usr/bin/env bash
# gen-contract-docs.sh
#
# Generate ABI / interface documentation for all contracts in contracts/*.
#
# What it does
# ------------
# 1. Runs `cargo doc` for every contract crate to produce rustdoc HTML.
# 2. Emits a machine-readable JSON summary of each contract's public interface
#    (functions + error codes) to docs/generated/contract-interfaces.json.
# 3. Exits non-zero if any step fails so CI can catch regressions.
#
# Usage
#   ./scripts/gen-contract-docs.sh            # generate docs
#   ./scripts/gen-contract-docs.sh --check    # verify JSON is up-to-date (CI mode)
#
# Requirements: cargo, jq (optional – only needed for pretty-printing)

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT_DIR="${REPO_ROOT}/docs/generated"
OUT_JSON="${OUT_DIR}/contract-interfaces.json"
CHECK_MODE=false
SKIP_RUSTDOC=${SKIP_RUSTDOC:-false}

if [[ "${1:-}" == "--check" ]]; then
  CHECK_MODE=true
fi

# ---------------------------------------------------------------------------
# Contract registry
# Each entry: "<crate-name>|<path-relative-to-repo-root>"
# ---------------------------------------------------------------------------
CONTRACTS=(
  "amm-pool|contracts/amm-pool"
  "governance-contract|contracts/governance"
  "multisig-wallet|contracts/multisig"
  "timelock|contracts/timelock"
  "reentrancy-guard|contracts/reentrancy-guard"
  "runtime-guard-wrapper|contracts/runtime-guard-wrapper"
  "my-contract|contracts/my-contract"
)
# ---------------------------------------------------------------------------
# Step 1 – rustdoc
# ---------------------------------------------------------------------------
if [[ "${SKIP_RUSTDOC}" == "1" || "${SKIP_RUSTDOC}" == "true" ]]; then
  echo "==> Skipping rustdoc (SKIP_RUSTDOC set)."
else
  echo "==> Generating rustdoc for contract crates..."
  for entry in "${CONTRACTS[@]}"; do
    crate="${entry%%|*}"
    path="${entry##*|}"
    echo "    cargo doc -p ${crate} --no-deps"
    cargo doc -p "${crate}" --no-deps --quiet 2>&1 || {
      echo "ERROR: cargo doc failed for ${crate}" >&2
      exit 1
    }
  done
  echo "    rustdoc output: ${REPO_ROOT}/target/doc/"
fi

# ---------------------------------------------------------------------------
# Step 2 – extract public interface summary from source
#
# We parse each lib.rs with grep/sed to extract:
#   - pub fn lines inside #[contractimpl] blocks
#   - #[contracterror] enum variants
#
# This is intentionally simple and dependency-free.  A richer tool (e.g.
# syn-based) can replace this later without changing the output schema.
# ---------------------------------------------------------------------------
echo ""
echo "==> Extracting public interface summary..."

mkdir -p "${OUT_DIR}"

# Build JSON incrementally
TMPFILE=$(mktemp)
echo "{" > "${TMPFILE}"
echo '  "schema_version": "1",' >> "${TMPFILE}"
echo '  "generated_at": "'"$(date -u +%Y-%m-%dT%H:%M:%SZ)"'",' >> "${TMPFILE}"
echo '  "contracts": {' >> "${TMPFILE}"

first_contract=true
for entry in "${CONTRACTS[@]}"; do
  crate="${entry%%|*}"
  path="${entry##*|}"
  lib="${REPO_ROOT}/${path}/src/lib.rs"
  # Use the directory name as the human-readable key in JSON
  contract_key="$(basename "${path}")"

  [[ -f "${lib}" ]] || { echo "WARN: ${lib} not found, skipping" >&2; continue; }

  # Extract public function names (lines matching `    pub fn <name>(`)
  fns_raw=$(grep -E '^\s+pub fn [a-z_]+\(' "${lib}" | \
    sed -E 's/.*pub fn ([a-z_]+)\(.*/\1/' | sort -u)

  # Extract error variant names from #[contracterror] enums
  errors_raw=$(awk '
    /#\[contracterror\]/{in_err=1; next}
    in_err && /^pub enum/{in_block=1; next}
    in_block && /^\}/{in_block=0; in_err=0; next}
    in_block && / = [0-9]+/{
      gsub(/[^A-Za-z].*/, "", $1); print $1
    }
  ' "${lib}" | grep -E '^[A-Z]' | sort -u || true)

  fn_count=0
  [[ -n "${fns_raw}" ]] && fn_count=$(echo "${fns_raw}" | wc -l | tr -d ' ')
  err_count=0
  [[ -n "${errors_raw}" ]] && err_count=$(echo "${errors_raw}" | wc -l | tr -d ' ')

  if [[ "${first_contract}" == "false" ]]; then
    echo "    ," >> "${TMPFILE}"
  fi
  first_contract=false

  echo "    \"${contract_key}\": {" >> "${TMPFILE}"
  echo '      "functions": [' >> "${TMPFILE}"
  fn_first=true
  while IFS= read -r fn; do
    [[ -z "${fn}" ]] && continue
    [[ "${fn_first}" == "false" ]] && echo ',' >> "${TMPFILE}"
    printf '        "%s"' "${fn}" >> "${TMPFILE}"
    fn_first=false
  done <<< "${fns_raw}" || true
  [[ "${fn_first}" == "false" ]] && echo "" >> "${TMPFILE}"
  echo '      ],' >> "${TMPFILE}"
  echo '      "errors": [' >> "${TMPFILE}"
  err_first=true
  while IFS= read -r err; do
    [[ -z "${err}" ]] && continue
    [[ "${err_first}" == "false" ]] && echo ',' >> "${TMPFILE}"
    printf '        "%s"' "${err}" >> "${TMPFILE}"
    err_first=false
  done <<< "${errors_raw}" || true
  [[ "${err_first}" == "false" ]] && echo "" >> "${TMPFILE}"
  echo '      ]' >> "${TMPFILE}"
  echo -n "    }" >> "${TMPFILE}"

  echo "    ${contract_key}: ${fn_count} function(s), ${err_count} error variant(s)"
done

echo "" >> "${TMPFILE}"
echo "  }" >> "${TMPFILE}"
echo "}" >> "${TMPFILE}"

# ---------------------------------------------------------------------------
# Step 3 – write or diff
# ---------------------------------------------------------------------------
if [[ "${CHECK_MODE}" == "true" ]]; then
  if [[ ! -f "${OUT_JSON}" ]]; then
    echo "ERROR: ${OUT_JSON} does not exist. Run gen-contract-docs.sh first." >&2
    rm -f "${TMPFILE}"
    exit 1
  fi
  # Compare ignoring the generated_at timestamp line
  if diff <(grep -v generated_at "${TMPFILE}") <(grep -v generated_at "${OUT_JSON}") > /dev/null 2>&1; then
    echo ""
    echo "==> Interface summary is up-to-date."
    rm -f "${TMPFILE}"
    exit 0
  else
    echo ""
    echo "ERROR: contract-interfaces.json is stale. Re-run gen-contract-docs.sh and commit the result." >&2
    diff <(grep -v generated_at "${TMPFILE}") <(grep -v generated_at "${OUT_JSON}") || true
    rm -f "${TMPFILE}"
    exit 1
  fi
fi

mv "${TMPFILE}" "${OUT_JSON}"
echo ""
echo "==> Written: ${OUT_JSON}"
echo "==> Done."
