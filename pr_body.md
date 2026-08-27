## Summary
This pull request addresses several issues across the Sanctifier contracts and tooling by introducing formal verification assertions, integration tests, and fixing formatting.

## Changes Included
* **Formal Verification (Reentrancy Guard)**: Added explicit invariants to `contracts/reentrancy-guard/src/lib.rs` that the Z3 solver backend can analyze.
* **Integration Tests (Main Tooling)**: Added integration tests for `tooling/sanctifier-cli/src/main.rs`, testing the CLI against mock workspaces.
* **Integration Tests (Configuration)**: Added integration tests for configuration processing (mocking `tooling/sanctifier-cli/src/config.rs`) to ensure SARIF/JSON outputs accurately reflect custom workspaces.
* **Documentation Formatting**: Restructured `README.md` to use better Markdown formatting, including tables and callouts.
* **Vulnerable Contract Syntax**: Fixed a missing delimiter in `contracts/vulnerable-contract/src/lib.rs` which was breaking `cargo fmt`.

## Related Issues
Closes #1401
Closes #1402
Closes #1403
Closes #1406
