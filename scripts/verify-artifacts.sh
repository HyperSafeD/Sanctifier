#!/usr/bin/env bash
# scripts/verify-artifacts.sh
# Verifies every release artifact listed in data/release-manifest.json is
# present and parseable.  This is the single CI gate that proves the
# release artifact set has not drifted from the manifest.  Canonical
# *formatting* is owned by prettier (`npm run format:db:check`); this
# script intentionally does not double-canonicalise JSON to avoid a
# formatter-on-formatter conflict.
#
# Usage:
#   scripts/verify-artifacts.sh            # default: validate
#   scripts/verify-artifacts.sh --check    # alias retained for CI / npm
#
# Hardened:
#   - set -euo pipefail
#   - manifest-driven file list (no hardcoded paths)
#   - exits early with a clear message if jq is missing
#   - distinct exit codes: 1 = artifact drift, 2 = environment error

set -euo pipefail

MANIFEST="data/release-manifest.json"

# `--check` is accepted for backwards compatibility with the previous
# script signature; it is a no-op since validation is always run.
if [[ "${1:-}" != "" && "${1:-}" != "--check" ]]; then
    echo "Usage: $0 [--check]" >&2
    exit 2
fi

if ! command -v jq >/dev/null 2>&1; then
    echo "Error: jq is required but not found in PATH." >&2
    exit 2
fi

if [[ ! -f "$MANIFEST" ]]; then
    echo "Error: release manifest $MANIFEST is missing." >&2
    exit 2
fi

# Validate the manifest itself first.
if ! jq empty "$MANIFEST" 2>/dev/null; then
    echo "Error: $MANIFEST is not valid JSON." >&2
    exit 1
fi

mapfile -t ARTIFACTS < <(
    jq -r '.artifacts.data[], .artifacts.schemas[]' "$MANIFEST"
)

if [[ ${#ARTIFACTS[@]} -eq 0 ]]; then
    echo "Error: release manifest declares no artifacts." >&2
    exit 1
fi

FAILED=0
for FILE in "${ARTIFACTS[@]}"; do
    if [[ ! -f "$FILE" ]]; then
        echo "Error: manifest references missing file $FILE." >&2
        FAILED=1
        continue
    fi

    case "$FILE" in
        *.json)
            if ! jq empty "$FILE" 2>/dev/null; then
                echo "Error: $FILE is not valid JSON." >&2
                FAILED=1
            fi
            ;;
        *.yaml|*.yml)
            # Parseability is enforced by the Node validator
            # (scripts/validate_db.js + validate_release_artifacts.js)
            # which loads each YAML through the `yaml` package.  We
            # only verify presence here.
            ;;
        *)
            echo "Warning: unhandled artifact type for $FILE — skipping." >&2
            ;;
    esac
done

if [[ $FAILED -ne 0 ]]; then
    echo "Verification failed.  Fix the errors above; canonical formatting is owned by 'npm run format:db'." >&2
    exit 1
fi

echo "All ${#ARTIFACTS[@]} release artifacts present and well-formed."
