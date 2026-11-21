#!/usr/bin/env bash
# scripts/test-provenance-path.sh
#
# Smoke test: assert that the output path hardcoded in
# scripts/generate-provenance.sh matches every path reference in
# docs/provenance-verification.md.
#
# A silent mismatch between the two files would mean the documented
# verification steps (grep -v '^#' CHECKSUMS.txt | sha256sum -c) silently
# fail to find the right file after a rename.  This script is intentionally
# dependency-free: it only uses bash built-ins and grep/sed.
#
# Exit codes:
#   0 — all paths consistent
#   1 — mismatch detected (details printed to stderr)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

GENERATE_SCRIPT="$REPO_ROOT/scripts/generate-provenance.sh"
DOCS_FILE="$REPO_ROOT/docs/provenance-verification.md"

# ── Helpers ──────────────────────────────────────────────────────────────────

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

pass() { echo -e "${GREEN}✓${NC} $*"; }
fail() { echo -e "${RED}✗${NC} $*" >&2; }
info() { echo -e "${YELLOW}→${NC} $*"; }

FAILED=0

# ── Preconditions ─────────────────────────────────────────────────────────────

if [[ ! -f "$GENERATE_SCRIPT" ]]; then
    fail "generate-provenance.sh not found at $GENERATE_SCRIPT"
    exit 1
fi

if [[ ! -f "$DOCS_FILE" ]]; then
    fail "provenance-verification.md not found at $DOCS_FILE"
    exit 1
fi

# ── Extract OUTPUT path from generate-provenance.sh ──────────────────────────
# Match the first assignment of the form: OUTPUT="..."  or  OUTPUT='...'
# (strips surrounding quotes)

SCRIPT_OUTPUT=$(grep -m1 '^OUTPUT=' "$GENERATE_SCRIPT" | sed 's/^OUTPUT=["'\'']\(.*\)["'\'']/\1/')

if [[ -z "$SCRIPT_OUTPUT" ]]; then
    fail "Could not find an OUTPUT= assignment in $GENERATE_SCRIPT"
    exit 1
fi

info "generate-provenance.sh declares OUTPUT=\"$SCRIPT_OUTPUT\""

# ── Extract every path-like token from docs that looks like our output file ───
# We look for every occurrence of the basename in the docs file.

BASENAME="${SCRIPT_OUTPUT##*/}"  # e.g. "CHECKSUMS.txt"

# Collect all distinct path-looking tokens containing the basename from docs.
# We capture both bare filenames and paths like dist/CHECKSUMS.txt.
DOC_REFS=()
while IFS= read -r line; do
    DOC_REFS+=("$line")
done < <(grep -oE '[A-Za-z0-9_./-]*'"$BASENAME"'[A-Za-z0-9_./-]*' "$DOCS_FILE" | sort -u)

if [[ ${#DOC_REFS[@]} -eq 0 ]]; then
    fail "docs/provenance-verification.md contains no reference to '$BASENAME'"
    fail "  Script output path: $SCRIPT_OUTPUT"
    fail "  Please update the docs to match the script, or vice versa."
    exit 1
fi

info "docs/provenance-verification.md references: ${DOC_REFS[*]}"

# ── Compare ───────────────────────────────────────────────────────────────────

MISMATCHES=0
for ref in "${DOC_REFS[@]}"; do
    # Normalise: strip leading ./ if present on either side before comparing.
    NORM_REF="${ref#./}"
    NORM_OUTPUT="${SCRIPT_OUTPUT#./}"

    if [[ "$NORM_REF" != "$NORM_OUTPUT" ]]; then
        fail "Path mismatch:"
        fail "  Script OUTPUT : $SCRIPT_OUTPUT"
        fail "  Docs reference: $ref"
        fail "  Update one to match the other so the documented verification"
        fail "  steps continue to find the generated file."
        MISMATCHES=$(( MISMATCHES + 1 ))
        FAILED=1
    fi
done

if [[ $MISMATCHES -eq 0 ]]; then
    pass "All $((${#DOC_REFS[@]})) doc reference(s) match the script OUTPUT path ($SCRIPT_OUTPUT)"
fi

# ── Summary ───────────────────────────────────────────────────────────────────

echo ""
if [[ $FAILED -ne 0 ]]; then
    echo -e "${RED}FAILED${NC}: provenance path consistency check failed."
    exit 1
else
    echo -e "${GREEN}PASSED${NC}: provenance path consistency check."
fi
