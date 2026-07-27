# Program Labels and Milestone Taxonomy

**Last Updated**: 2026-07-27

This document defines the label and milestone conventions used for the Mainnet Launch Readiness and ZK Integration waves (#1112-#1261+).

---

## Milestones

### Active Milestones

**1. Mainnet Launch Readiness**
- **Scope**: Security hardening, deployment operations, reliability, compliance work required before mainnet launch
- **Issues**: #1112-#1189 (A1-A7 workstreams)
- **Tracking**: #1252 (epic)
- **Timeline**: Current wave priority

**2. ZK Integration**
- **Scope**: Zero-knowledge proof verification rules (Z001-Z014), circom/Noir parser integration, ZK-specific fixtures and documentation
- **Issues**: #1190-#1261
- **Timeline**: Parallel to mainnet readiness
- **Dependencies**: Some ZK rules depend on #1192, #1194 infrastructure

**3. Next Wave — Program Ops**
- **Scope**: Post-launch operational improvements, distribution, community features, and deferred enhancements
- **Timeline**: After mainnet launch

---

## Label Taxonomy

### Thematic Labels (This Wave)

**`mainnet`**
- Applied to issues blocking mainnet launch
- Must be completed before cutting mainnet-ready release
- Cross-references Mainnet Launch Readiness milestone

**`zk`**
- Applied to zero-knowledge specific features/rules
- Includes Z001-Z014 rules, parsers, ZK fixtures
- Cross-references ZK Integration milestone

**`next-wave`**
- Applied to issues deferred to post-launch
- Good ideas that don't block mainnet
- Cross-references Next Wave milestone

### Component Labels

**Format**: `component:*`

Examples:
- `component:core` - Sanctifier core analysis engine
- `component:cli` - Command-line interface
- `component:dashboard` - Web dashboard frontend
- `component:ci` - CI/CD pipelines
- `component:docs` - Documentation
- `component:contracts` - Example/test contracts

### Type Labels

**Format**: `type:*`

- `type:feature` - New functionality
- `type:bug` - Something broken
- `type:enhancement` - Improvement to existing feature
- `type:docs` - Documentation work
- `type:chore` - Maintenance/cleanup
- `type:epic` - Tracking issue aggregating multiple issues

### Priority Labels

**Format**: `priority:p*`

- `priority:p0` - Blocker (mainnet launch blocked)
- `priority:p1` - Critical (must have for mainnet)
- `priority:p2` - Important (should have)
- `priority:p3` - Nice to have (can defer)

### Difficulty Labels

**Format**: `difficulty:*`

- `difficulty:easy` - <1 day
- `difficulty:medium` - 2-5 days
- `difficulty:hard` - 1-2 weeks
- `difficulty:expert` - 2+ weeks or requires specialized knowledge

---

## Triage Guidelines

### For New Issues

1. **Choose milestone**:
   - Mainnet Launch Readiness: Security/ops/reliability blocking launch
   - ZK Integration: Zero-knowledge specific work
   - Next Wave: Post-launch improvements

2. **Add thematic label**:
   - `mainnet` if blocking launch
   - `zk` if ZK-specific
   - `next-wave` if deferred

3. **Add component label**: Which part of codebase affected

4. **Add type label**: Feature, bug, enhancement, etc.

5. **Add priority**: p0 (blocker) → p3 (nice to have)

6. **Add difficulty** (optional): Helps with planning

### Example

Issue: "Add Z001 nullifier detection rule"

- **Milestone**: ZK Integration
- **Labels**: `zk`, `component:core`, `type:feature`, `priority:p1`, `difficulty:hard`

---

## Workstream Codes (Mainnet Readiness)

Issues #1112-#1189 use workstream prefixes:

- **A1**: Security hardening
- **A2**: Contract hardening  
- **A3**: Deployment operations
- **A4**: Reliability & monitoring
- **A5**: Compliance & auditing
- **A6**: Distribution & packaging
- **A7**: Testing & validation

See #1252 for complete workstream breakdown.

---

## Label History Notes

**Previous Conventions** (for context):
- Some older issues use `area:*` instead of `component:*`
- Some use space-separated labels vs hyphenated
- This document standardizes on current conventions

**Migration**: Not retroactively relabeling old issues; new issues follow this taxonomy.

---

## Contributing

When filing new issues:

1. Read this document
2. Choose appropriate milestone and labels
3. Use existing label names (don't create new ones without discussion)
4. For mainnet-blocking work, add `mainnet` label and link to #1252

Questions? See CONTRIBUTING.md or ask in discussions.
