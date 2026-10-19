# Release Process Documentation

**Issue #679**: Add unit tests + fixtures for release process documentation

## Overview

This document defines the complete release process for Sanctifier, ensuring reliable, predictable, and safe releases.

## Versioning Scheme

Sanctifier follows **Semantic Versioning 2.0.0**:

```
MAJOR.MINOR.PATCH[-PRERELEASE][+BUILD]

Examples:
- 1.0.0         (initial release)
- 1.1.0         (minor feature addition)
- 1.1.1         (patch/bugfix)
- 2.0.0-rc.1    (release candidate)
- 2.0.0-beta.1  (beta release)
```

### Version Bump Rules

- **MAJOR**: Breaking API changes, security fixes requiring migration
- **MINOR**: New features, non-breaking enhancements
- **PATCH**: Bug fixes, documentation updates, performance improvements

## Release Checklist

### Pre-Release (3 days before)

- [ ] Create release branch: `release/v{VERSION}`
- [ ] Update `CHANGELOG.md` with all changes since last release
- [ ] Update version in `Cargo.toml`, `package.json`
- [ ] Review all commits since last release
- [ ] Run full test suite: `cargo test --all`
- [ ] Run security checks: `cargo audit`
- [ ] Build all contracts for testnet
- [ ] Generate API documentation

### Testing Phase (2 days before)

- [ ] Run integration tests on testnet
- [ ] Verify contract deployments work
- [ ] Test frontend against testnet contracts
- [ ] Run load tests (if applicable)
- [ ] Security review of changes
- [ ] Documentation review

### Release Day

- [ ] Create annotated git tag: `git tag -a v{VERSION} -m "Release {VERSION}"`
- [ ] Push tag to upstream: `git push origin v{VERSION}`
- [ ] Create GitHub release with changelog
- [ ] Build final release artifacts
- [ ] Publish to crates.io (if applicable)
- [ ] Deploy to production (if automated)
- [ ] Verify production deployment
- [ ] Announce release on forums/socials

### Post-Release (1 day after)

- [ ] Monitor production metrics
- [ ] Respond to early user feedback
- [ ] Prepare patch release if critical issues found
- [ ] Document any deployment issues
- [ ] Archive release artifacts

## Testing Requirements

Before any release, ensure:

1. **Unit Tests**: `cargo test --lib`
   - Minimum 80% code coverage
   - All error paths tested
   - Edge cases covered

2. **Integration Tests**: `cargo test --test '*'`
   - End-to-end workflows
   - Contract deployment and execution
   - API functionality
   - Error handling

3. **Contract Tests**: `cd contracts && cargo test --all`
   - Each contract built and tested
   - Testnet deployment validated
   - Cross-contract interactions (if applicable)

4. **Frontend Tests**: `cd frontend && npm run test`
   - Unit tests pass
   - Integration tests pass
   - E2E tests on testnet

5. **Security Tests**: `cargo audit`, `npm audit`
   - No high/critical vulnerabilities
   - Dependencies up to date
   - License compliance check

## Rollback Plan

If a critical issue is discovered post-release:

1. **Immediate**: Announce on security channels
2. **Within 1 hour**: Publish patch release (e.g., 1.0.1)
3. **Instructions**: Provide downgrade instructions to users
4. **Post-mortem**: Document incident and prevention measures

## Release Communication

### Changelog Format

```markdown
## [1.0.0] - 2024-04-25

### Added
- New feature description
- API endpoint addition

### Fixed
- Bug fix description
- Security issue fixed

### Changed
- Breaking change description
- Dependency updates

### Security
- Security advisory (if applicable)

### Deprecated
- API deprecation notices
```

### Release Notes Template

```markdown
# Release v{VERSION}

**Release Date**: YYYY-MM-DD

## Summary
Brief overview of this release's focus and key achievements.

## Major Features
- Feature 1 description
- Feature 2 description

## Bug Fixes
- Bug fix 1
- Bug fix 2

## Security Updates
- Security fix 1 (CVE if applicable)

## Breaking Changes
- Breaking change with migration guide

## Upgrade Instructions
Steps for users to upgrade from previous version.

## Contributors
- @contributor1
- @contributor2

## Links
- [Full Changelog](../CHANGELOG.md)
- [Documentation](../DOCUMENTATION_INDEX.md)
```

## Automation

The release process is partially automated:

1. **CI/CD Pipeline**: Runs on every tag push
2. **Automated Tests**: All test suites execute
3. **Artifact Building**: Contracts and binaries built
4. **GitHub Release**: Automatically created from annotated tags

See `.github/workflows/release.yml` for automation details.

## Support

For release-related questions:
- See [DOCUMENTATION_INDEX.md](./DOCUMENTATION_INDEX.md)
- Review previous releases: [GitHub Releases](https://github.com/HyperSafeD/Sanctifier/releases)
- Contact maintainers on security channel for security-related releases
