# vulnerable-contract

⚠️ **INTENTIONALLY VULNERABLE. DO NOT DEPLOY.**

This crate is a test fixture for Sanctifier's analyzers (S001 missing auth,
S003 unchecked arithmetic, unhandled panics, etc.). Every "❌" function is
insecure by design, and even the "✅" variants are simplified demonstrations,
not production patterns. For example, `set_admin_secure` does not actually
enforce auth: the admin is a `Symbol` rather than an `Address`, so the
`require_auth` call is commented out.

## Safeguards

- The crate compiles to an empty library unless built with
  `--features test-fixtures` (or under `cargo test`).
- It is excluded from the workspace's `default-members`, so a plain
  `cargo build` skips it.
- `publish = false` prevents accidental publishing to crates.io.

## Usage

```
cargo test -p vulnerable-contract
cargo build -p vulnerable-contract --features test-fixtures
```
