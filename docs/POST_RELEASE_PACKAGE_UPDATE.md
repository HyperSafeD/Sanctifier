# Post-Release Package Update Guide

**Related Issue**: #1166  
**Purpose**: Manual steps to update Scoop and WinGet manifests after a release is published

## Overview

After a new version is tagged and the GitHub Release workflow completes:
- ✅ **Homebrew** - Auto-updated by CI
- ✅ **npm** - Auto-published by CI
- ⚠️ **Scoop** - Requires manual checksum update
- ⚠️ **WinGet** - Requires manual PR to microsoft/winget-pkgs

This guide covers the manual steps for Scoop and WinGet.

---

## Prerequisites

1. Release has been published to GitHub with all binary artifacts
2. `SHA256SUMS` file is available in the release
3. You have write access to the HyperSafeD/Sanctifier repository

---

## Step 1: Download SHA256SUMS

```bash
# Set the version you're updating to
VERSION="0.1.0"
TAG="v${VERSION}"

# Download checksums
gh release download "$TAG" --repo HyperSafeD/Sanctifier --pattern 'SHA256SUMS'

# View the checksums
cat SHA256SUMS
```

---

## Step 2: Update Scoop Manifest

### Extract Windows Checksums

```bash
# Extract checksums for Windows binaries
AMD64_SHA=$(grep "sanctifier-windows-amd64.exe" SHA256SUMS | awk '{print $1}')
ARM64_SHA=$(grep "sanctifier-windows-arm64.exe" SHA256SUMS | awk '{print $1}')

echo "AMD64: $AMD64_SHA"
echo "ARM64: $ARM64_SHA"
```

### Update `scoop/sanctifier.json`

Replace the placeholder checksums:

```json
{
  "version": "0.1.0",
  "architecture": {
    "64bit": {
      "url": "https://github.com/HyperSafeD/Sanctifier/releases/download/v0.1.0/sanctifier-windows-amd64.exe",
      "hash": "<AMD64_SHA_HERE>"
    },
    "arm64": {
      "url": "https://github.com/HyperSafeD/Sanctifier/releases/download/v0.1.0/sanctifier-windows-arm64.exe",
      "hash": "<ARM64_SHA_HERE>"
    }
  }
}
```

### Automated Update Script

```bash
#!/bin/bash
# Update Scoop manifest checksums

VERSION="0.1.0"
AMD64_SHA="<your-amd64-sha>"
ARM64_SHA="<your-arm64-sha>"

sed -i.bak \
  -e "s/\"version\": \".*\"/\"version\": \"$VERSION\"/" \
  -e "0,/\"hash\": \".*\"/{s/\"hash\": \".*\"/\"hash\": \"$AMD64_SHA\"/}" \
  -e "0,/\"hash\": \".*\"/{s/\"hash\": \".*\"/\"hash\": \"$ARM64_SHA\"/;}" \
  scoop/sanctifier.json

rm scoop/sanctifier.json.bak
```

### Test Scoop Locally (Windows)

```powershell
# Install from local manifest
scoop install ./scoop/sanctifier.json

# Verify
sanctifier --version
```

### Commit and Push

```bash
git checkout -b fix/update-scoop-checksums-v$VERSION
git add scoop/sanctifier.json
git commit -m "chore(scoop): update checksums for v$VERSION"
git push origin fix/update-scoop-checksums-v$VERSION

# Create PR
gh pr create --title "chore(scoop): update checksums for v$VERSION" \
  --body "Updates Scoop manifest checksums for release v$VERSION

- AMD64: $AMD64_SHA
- ARM64: $ARM64_SHA

Closes #1166 (Scoop portion)"
```

---

## Step 3: Update WinGet Manifests

### Extract Windows Checksums

Same as above:

```bash
AMD64_SHA=$(grep "sanctifier-windows-amd64.exe" SHA256SUMS | awk '{print $1}')
ARM64_SHA=$(grep "sanctifier-windows-arm64.exe" SHA256SUMS | awk '{print $1}')
```

### Update Installer Manifest

File: `winget/manifests/h/HyperSafeD/Sanctifier/0.1.0/HyperSafeD.Sanctifier.installer.yaml`

Replace placeholder checksums:

```yaml
PackageIdentifier: HyperSafeD.Sanctifier
PackageVersion: 0.1.0
InstallerLocale: en-US
InstallerType: portable
Commands:
  - sanctifier
Installers:
  - Architecture: x64
    InstallerUrl: https://github.com/HyperSafeD/Sanctifier/releases/download/v0.1.0/sanctifier-windows-amd64.exe
    InstallerSha256: <AMD64_SHA_HERE>
    UpgradeBehavior: uninstallPrevious
  - Architecture: arm64
    InstallerUrl: https://github.com/HyperSafeD/Sanctifier/releases/download/v0.1.0/sanctifier-windows-arm64.exe
    InstallerSha256: <ARM64_SHA_HERE>
    UpgradeBehavior: uninstallPrevious
ManifestType: installer
ManifestVersion: 1.6.0
```

### Test Locally (Windows)

```powershell
# Validate manifest
winget validate --manifest winget/manifests/h/HyperSafeD/Sanctifier/0.1.0/

# Test installation from local manifest
winget install --manifest winget/manifests/h/HyperSafeD/Sanctifier/0.1.0/HyperSafeD.Sanctifier.yaml

# Verify
sanctifier --version
```

### Submit to microsoft/winget-pkgs

WinGet packages must be submitted to the official repository.

#### Option 1: Manual PR

1. Fork https://github.com/microsoft/winget-pkgs
2. Create the manifest path: `manifests/h/HyperSafeD/Sanctifier/0.1.0/`
3. Copy your three manifest files:
   - `HyperSafeD.Sanctifier.yaml`
   - `HyperSafeD.Sanctifier.installer.yaml`
   - `HyperSafeD.Sanctifier.locale.en-US.yaml`
4. Create PR to microsoft/winget-pkgs

#### Option 2: Using WinGet Submit Tool

```powershell
# Install wingetcreate
winget install Microsoft.WingetCreate

# Submit update
wingetcreate update HyperSafeD.Sanctifier --version 0.1.0 --submit --token <GITHUB_TOKEN>
```

The tool will:
- Auto-download binaries
- Calculate checksums
- Generate updated manifests
- Create PR to microsoft/winget-pkgs automatically

#### Option 3: Using CLI

```bash
# Using winget-pkgs-submission-tool
npx @microsoft/winget-create update HyperSafeD.Sanctifier \
  --version 0.1.0 \
  --urls \
    "https://github.com/HyperSafeD/Sanctifier/releases/download/v0.1.0/sanctifier-windows-amd64.exe|x64" \
    "https://github.com/HyperSafeD/Sanctifier/releases/download/v0.1.0/sanctifier-windows-arm64.exe|arm64" \
  --submit --token $GITHUB_TOKEN
```

### Wait for Review

- WinGet PRs are reviewed by Microsoft maintainers
- Usually takes 1-3 days
- Automated validations run first
- Once merged, package is available in WinGet

---

## Step 4: Update Main Repository

Once Scoop and WinGet are updated:

```bash
git checkout -b fix/update-package-checksums-v$VERSION
git add scoop/sanctifier.json
git add winget/manifests/
git commit -m "chore: update package checksums for v$VERSION

- Updated Scoop manifest with real SHA256 checksums
- Updated WinGet manifest with real SHA256 checksums
- Verified all four packaging channels working end-to-end

Closes #1166"

git push origin fix/update-package-checksums-v$VERSION

gh pr create --title "chore: update package checksums for v$VERSION" \
  --body "Final package QA update for #1166

## Summary
Updated Scoop and WinGet manifests with production checksums for v$VERSION release.

## Changes
- ✅ Scoop: Updated SHA256 for AMD64 and ARM64
- ✅ WinGet: Updated SHA256 for AMD64 and ARM64
- ✅ Tested all installation methods end-to-end

## Testing
- [x] Homebrew: Verified auto-update worked
- [x] npm: Verified auto-publish worked
- [x] Scoop: Manually tested installation
- [x] WinGet: Submitted PR to microsoft/winget-pkgs

## Related
Closes #1166"
```

---

## Step 5: Verification

Run the QA test script:

```bash
./scripts/test-packages.sh 0.1.0
```

This will test all four packaging channels and verify:
- ✅ Correct version is installed
- ✅ Checksums match SHA256SUMS
- ✅ Basic functionality works

---

## Troubleshooting

### Scoop: Hash Mismatch

If Scoop reports a hash mismatch:

```powershell
# Clear Scoop cache
scoop cache rm sanctifier

# Verify checksum manually
certutil -hashfile <path-to-exe> SHA256
```

### WinGet: Validation Failed

Common validation errors:

1. **Invalid SHA256**: Ensure uppercase hex, 64 characters
2. **URL not accessible**: Wait for release to be fully published
3. **Version mismatch**: Ensure all three manifest files have same version

### npm: Version Not Published

Check npm registry:

```bash
npm view @hypersafed/sanctifier-cli versions
```

If missing, the `npm-publish` workflow may have failed. Check GitHub Actions.

### Homebrew: Formula Not Updated

Check the homebrew-tap repository:

```bash
gh repo view HyperSafeD/homebrew-sanctifier

# View recent commits
git log --oneline -5
```

If not updated, the `homebrew-update` workflow may have failed.

---

## Automation Opportunities

Future improvements to reduce manual work:

1. **Scoop**: Add workflow to auto-commit updated checksums
2. **WinGet**: Use `wingetcreate` in CI to auto-submit
3. **Checksums**: Add to main `CHECKSUMS.txt` during build
4. **Testing**: Run package tests in CI before release

---

## Checklist

After completing this guide:

- [ ] Downloaded `SHA256SUMS` from release
- [ ] Updated `scoop/sanctifier.json` with real checksums
- [ ] Tested Scoop installation locally (if Windows available)
- [ ] Updated `winget/manifests/` with real checksums
- [ ] Submitted WinGet PR to microsoft/winget-pkgs
- [ ] Created PR to main repo with updated manifests
- [ ] Ran `scripts/test-packages.sh` successfully
- [ ] Updated `QA_PACKAGE_VERIFICATION_REPORT.md` with results
- [ ] Commented on #1166 with verification results

---

## References

- [Scoop Manifest Reference](https://github.com/ScoopInstaller/Scoop/wiki/App-Manifests)
- [WinGet Manifest Schema](https://github.com/microsoft/winget-cli/blob/master/schemas/JSON/manifests/v1.6.0/manifest.installer.1.6.0.json)
- [WinGet Submit Guide](https://github.com/microsoft/winget-pkgs/blob/master/CONTRIBUTING.md)
- [Homebrew Formula Cookbook](https://docs.brew.sh/Formula-Cookbook)
