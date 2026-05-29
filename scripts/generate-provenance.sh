#!/usr/bin/env bash
# scripts/generate-provenance.sh
# Generates CHECKSUMS.txt covering every release artifact listed in
# data/release-manifest.json.  Atomic-write semantics: the manifest is
# constructed in a temp file and only moved into place once every hash
# has succeeded — a partial CHECKSUMS.txt cannot leak on interrupt.
#
# Hardened:
#   - set -euo pipefail
#   - manifest-driven file list (single source of truth)
#   - portable: prefers sha256sum (Linux), falls back to `shasum -a 256`
#     (macOS) — fails fast if neither is present
#   - atomic install of the final manifest via `mv`
#   - reproducible: paths are emitted relative to repo root, sorted by
#     manifest order (deterministic)

set -euo pipefail

MANIFEST="data/release-manifest.json"
OUTPUT="CHECKSUMS.txt"

if ! command -v jq >/dev/null 2>&1; then
    echo "Error: jq is required but not found in PATH." >&2
    exit 2
fi

if [[ ! -f "$MANIFEST" ]]; then
    echo "Error: release manifest $MANIFEST is missing." >&2
    exit 2
fi

# Pick a deterministic SHA-256 implementation.
if command -v sha256sum >/dev/null 2>&1; then
    SHA_CMD=(sha256sum)
elif command -v shasum >/dev/null 2>&1; then
    SHA_CMD=(shasum -a 256)
else
    echo "Error: neither sha256sum nor shasum is available." >&2
    exit 2
fi

# Always run the formatter first so the hashes match the canonical
# on-disk form.  We do NOT pass --check here: this script is the
# release-time path that may auto-fix formatting before hashing.
./scripts/verify-artifacts.sh

mapfile -t ARTIFACTS < <(
    jq -r '.artifacts.data[], .artifacts.schemas[]' "$MANIFEST"
)

if [[ ${#ARTIFACTS[@]} -eq 0 ]]; then
    echo "Error: release manifest declares no artifacts." >&2
    exit 2
fi

# Build the manifest in a temp file; install atomically at the end.
TMP_OUT=$(mktemp)
trap 'rm -f "$TMP_OUT"' EXIT

{
    echo "# Sanctifier Release Artifact Checksums (SHA-256)"
    echo "# Generated on $(date -u +'%Y-%m-%dT%H:%M:%SZ')"
    echo "# Source manifest: $MANIFEST"
    echo "# Verify locally: grep -v '^#' CHECKSUMS.txt | sha256sum -c"
    echo ""
} > "$TMP_OUT"

for FILE in "${ARTIFACTS[@]}"; do
    if [[ ! -f "$FILE" ]]; then
        echo "Error: manifest references missing file $FILE." >&2
        exit 1
    fi
    "${SHA_CMD[@]}" "$FILE" >> "$TMP_OUT"
done

mv "$TMP_OUT" "$OUTPUT"
trap - EXIT

echo "Provenance manifest generated at $OUTPUT (${#ARTIFACTS[@]} artifacts)."
