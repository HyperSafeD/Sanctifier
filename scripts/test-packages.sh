#!/usr/bin/env bash
#
# Package QA Testing Script for Issue #1166
# Tests all four packaging channels against a released version
#
# Usage: ./scripts/test-packages.sh <version>
# Example: ./scripts/test-packages.sh 0.1.0
#

set -euo pipefail

VERSION="${1:-}"
if [ -z "$VERSION" ]; then
  echo "Usage: $0 <version>"
  echo "Example: $0 0.1.0"
  exit 1
fi

TAG="v${VERSION}"
REPO="HyperSafeD/Sanctifier"
PASS="\033[0;32m✅ PASS\033[0m"
FAIL="\033[0;31m❌ FAIL\033[0m"
SKIP="\033[0;33m⏭️  SKIP\033[0m"
INFO="\033[0;36mℹ️  INFO\033[0m"

echo "========================================"
echo "Package QA Test Suite - Issue #1166"
echo "========================================"
echo "Version: $VERSION"
echo "Tag: $TAG"
echo "Repository: $REPO"
echo ""

# Function to check if command exists
command_exists() {
  command -v "$1" >/dev/null 2>&1
}

# Function to compare version output
check_version() {
  local actual="$1"
  local expected="$VERSION"
  if echo "$actual" | grep -q "$expected"; then
    echo -e "$PASS Version matches: $actual"
    return 0
  else
    echo -e "$FAIL Version mismatch: expected '$expected', got '$actual'"
    return 1
  fi
}

# Download SHA256SUMS from release
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "Step 1: Download SHA256SUMS from release"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

if command_exists gh; then
  echo -e "$INFO Downloading SHA256SUMS..."
  gh release download "$TAG" --repo "$REPO" --pattern 'SHA256SUMS' --clobber || {
    echo -e "$FAIL Could not download SHA256SUMS"
    exit 1
  }
  echo -e "$PASS Downloaded SHA256SUMS"
else
  echo -e "$SKIP gh CLI not found, skipping SHA256SUMS download"
fi
echo ""

# Test Homebrew
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "Step 2: Test Homebrew"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

if command_exists brew; then
  echo -e "$INFO Testing Homebrew installation..."
  
  # Uninstall if already installed
  brew uninstall sanctifier 2>/dev/null || true
  
  # Tap if not already tapped
  brew tap HyperSafeD/sanctifier 2>/dev/null || true
  
  # Install
  if brew install sanctifier; then
    echo -e "$PASS brew install succeeded"
  else
    echo -e "$FAIL brew install failed"
    exit 1
  fi
  
  # Check version
  version_output=$(sanctifier --version 2>&1 || echo "ERROR")
  check_version "$version_output"
  
  # Check binary location
  brew_bin=$(which sanctifier)
  echo -e "$INFO Binary location: $brew_bin"
  
  # Verify checksum if SHA256SUMS exists
  if [ -f SHA256SUMS ]; then
    echo -e "$INFO Verifying checksum..."
    if [[ "$OSTYPE" == "darwin"* ]]; then
      actual_sha=$(shasum -a 256 "$brew_bin" | awk '{print $1}')
    else
      actual_sha=$(sha256sum "$brew_bin" | awk '{print $1}')
    fi
    echo -e "$INFO SHA256: $actual_sha"
    if grep -q "$actual_sha" SHA256SUMS; then
      echo -e "$PASS Checksum verified"
    else
      echo -e "$FAIL Checksum not found in SHA256SUMS"
    fi
  fi
  
  # Test basic functionality
  if sanctifier --help >/dev/null 2>&1; then
    echo -e "$PASS sanctifier --help works"
  else
    echo -e "$FAIL sanctifier --help failed"
  fi
  
else
  echo -e "$SKIP Homebrew not installed (macOS/Linux only)"
fi
echo ""

# Test npm
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "Step 3: Test npm"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

if command_exists npm; then
  echo -e "$INFO Testing npm installation..."
  
  # Uninstall if already installed
  npm uninstall -g @hypersafed/sanctifier-cli 2>/dev/null || true
  
  # Clear cache
  rm -rf ~/.sanctifier/bin 2>/dev/null || true
  
  # Install
  if npm install -g @hypersafed/sanctifier-cli; then
    echo -e "$PASS npm install succeeded"
  else
    echo -e "$FAIL npm install failed"
    exit 1
  fi
  
  # Check version
  version_output=$(sanctifier --version 2>&1 || echo "ERROR")
  check_version "$version_output"
  
  # Verify binary downloads correctly
  echo -e "$INFO Testing binary download..."
  rm -rf ~/.sanctifier/bin
  if sanctifier --version >/dev/null 2>&1; then
    echo -e "$PASS Binary auto-download works"
  else
    echo -e "$FAIL Binary auto-download failed"
  fi
  
  # Test basic functionality
  if sanctifier --help >/dev/null 2>&1; then
    echo -e "$PASS sanctifier --help works"
  else
    echo -e "$FAIL sanctifier --help failed"
  fi
  
else
  echo -e "$SKIP npm not installed"
fi
echo ""

# Test Scoop (Windows only)
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "Step 4: Test Scoop"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

if command_exists scoop; then
  echo -e "$INFO Testing Scoop installation..."
  
  # Uninstall if already installed
  scoop uninstall sanctifier 2>/dev/null || true
  
  # Add bucket if not added
  scoop bucket add hypersafed https://github.com/HyperSafeD/scoop-bucket 2>/dev/null || true
  
  # Install
  if scoop install sanctifier; then
    echo -e "$PASS scoop install succeeded"
  else
    echo -e "$FAIL scoop install failed"
    exit 1
  fi
  
  # Check version
  version_output=$(sanctifier --version 2>&1 || echo "ERROR")
  check_version "$version_output"
  
  # Test basic functionality
  if sanctifier --help >/dev/null 2>&1; then
    echo -e "$PASS sanctifier --help works"
  else
    echo -e "$FAIL sanctifier --help failed"
  fi
  
else
  echo -e "$SKIP Scoop not installed (Windows only)"
fi
echo ""

# Test WinGet (Windows only)
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "Step 5: Test WinGet"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

if command_exists winget; then
  echo -e "$INFO Testing WinGet installation..."
  
  # Uninstall if already installed
  winget uninstall HyperSafeD.Sanctifier 2>/dev/null || true
  
  # Install
  if winget install HyperSafeD.Sanctifier --version "$VERSION"; then
    echo -e "$PASS winget install succeeded"
  else
    echo -e "$FAIL winget install failed (may need manual manifest submission)"
    echo -e "$INFO WinGet requires PR to microsoft/winget-pkgs"
  fi
  
  # Check version
  version_output=$(sanctifier --version 2>&1 || echo "ERROR")
  check_version "$version_output"
  
  # Verify installation
  if winget list | grep -q "Sanctifier"; then
    echo -e "$PASS Sanctifier found in winget list"
  else
    echo -e "$FAIL Sanctifier not found in winget list"
  fi
  
  # Test basic functionality
  if sanctifier --help >/dev/null 2>&1; then
    echo -e "$PASS sanctifier --help works"
  else
    echo -e "$FAIL sanctifier --help failed"
  fi
  
else
  echo -e "$SKIP WinGet not installed (Windows only)"
fi
echo ""

# Summary
echo "========================================"
echo "QA Test Summary"
echo "========================================"
echo ""
echo "✅ All available packaging channels tested"
echo ""
echo "Next Steps:"
echo "  1. Review test results above"
echo "  2. Update QA_PACKAGE_VERIFICATION_REPORT.md with findings"
echo "  3. Fix any issues found"
echo "  4. Re-run tests until all pass"
echo ""
echo "Note: Scoop and WinGet require manual checksum updates"
echo "      after the release is published."
echo ""

# Cleanup
echo -e "$INFO Cleaning up..."
rm -f SHA256SUMS

echo "✅ QA Test Complete!"
