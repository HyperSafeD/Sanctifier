#!/usr/bin/env bash
set -euo pipefail

VERSION="${1:-}"
if [ -z "$VERSION" ]; then
  echo "Usage: $0 <version>"
  echo "Example: $0 0.2.0"
  exit 1
fi

if ! echo "$VERSION" | grep -qE '^[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9.]+)?$'; then
  echo "Error: version must be in semver format (e.g. 0.2.0, 0.2.0-rc1)"
  exit 1
fi

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

echo "Bumping version to $VERSION"

prev_version=$(grep -E '^version\s*=' tooling/sanctifier-cli/Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/')
echo "  Current version: $prev_version"

bump_cargo_toml() {
  local file="$1"
  if grep -q '^version = ' "$file"; then
    sed -i.bak "s/^version = \".*\"/version = \"$VERSION\"/" "$file" && rm -f "$file.bak"
    echo "  Updated $file"
  fi
}

bump_cargo_dep_version() {
  local file="$1"
  local dep="$2"
  if grep -q "$dep" "$file"; then
    sed -i.bak "s/\\($dep.*version = \"\\)[^\"]*\"/\\1$VERSION\"/" "$file" && rm -f "$file.bak"
    echo "  Updated $dep dependency in $file"
  fi
}

bump_package_json() {
  local file="$1"
  if grep -q '"version"' "$file"; then
    sed -i.bak 's/"version": "[^"]*"/"version": "'"$VERSION"'"/' "$file" && rm -f "$file.bak"
    echo "  Updated $file"
  fi
}

bump_homebrew_formula() {
  local file="$1"
  sed -i.bak "s/version \"[^\"]*\"/version \"$VERSION\"/" "$file" && rm -f "$file.bak"
  echo "  Updated $file"
}

bump_cargo_toml "Cargo.toml"
bump_cargo_toml "tooling/sanctifier-cli/Cargo.toml"
bump_cargo_toml "tooling/sanctifier-core/Cargo.toml"
bump_cargo_toml "tooling/sanctifier-detector/Cargo.toml"
bump_cargo_toml "tooling/sanctifier-wasm/Cargo.toml"

bump_cargo_dep_version "tooling/sanctifier-cli/Cargo.toml" "sanctifier-core"

bump_package_json "vscode-extension/package.json"
bump_package_json "packages/sanctifier-cli-npm/package.json"

bump_homebrew_formula "homebrew/sanctifier.rb"

echo ""
echo "Version bump complete: $prev_version → $VERSION"
echo ""
echo "Next steps:"
echo "  1. Review the changes: git diff"
echo "  2. Update CHANGELOG.md — move Unreleased entries to a new [$VERSION] section"
echo "  3. Commit: git commit -am 'chore: bump version to $VERSION'"
echo "  4. Tag:    git tag -a v$VERSION -m 'Release v$VERSION'"
echo "  5. Push:   git push origin main && git push origin v$VERSION"
