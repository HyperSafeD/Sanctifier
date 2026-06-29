Closes #1055

### Summary
Adds an end-to-end integration test workflow `.github/workflows/test-action.yml` for the GitHub composite action defined in `action.yml`.

### Changes
1. **Added `.github/workflows/test-action.yml`**:
   - Runs on every push and pull request to the main branch.
   - Tests the action across a matrix of 3 operating systems: `ubuntu-latest`, `macos-latest`, `windows-latest`.
   - Setup dependencies (stable Rust, Python 3.x, Z3 solver) required by `sanctifier-cli`.
   - Executes the composite action `./` against the fixture contract at `contracts/fixtures/auth_gap_contract.rs`.
   - Asserts that the action exits with code 1 (indicating security findings were successfully detected).
   - Asserts that the action produces a valid SARIF output file (`test-output.sarif`).
2. **Added `contracts/fixtures/auth_gap_contract.rs`**:
   - Copy of `tests/fixtures/auth_gap_contract.rs` to serve as a reliable fixture contract containing auth gap issues for the action to analyze.
