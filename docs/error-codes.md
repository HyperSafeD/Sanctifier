# Sanctifier Error Code Mapping

Sanctifier uses a unified finding code system across `sanctifier-core` and `sanctifier-cli` outputs.

| Code | Category | Meaning |
|------|----------|---------|
| `S001` | authentication | Missing `require_auth` in a state-changing function |
| `S002` | panic_handling | `panic!` / `unwrap` / `expect` usage that may abort execution |
| `S003` | arithmetic | Unchecked arithmetic with overflow/underflow risk |
| `S004` | storage_limits | Ledger entry size exceeds or approaches the configured threshold |
| `S005` | storage_keys | Potential storage-key collision across data paths |
| `S006` | unsafe_patterns | Potentially unsafe language or runtime pattern detected |
| `S007` | custom_rule | User-defined rule matched contract source |
| `S008` | events | Inconsistent topic counts or sub-optimal gas patterns in events |
| `S009` | logic | A `Result` return value is not consumed or handled |
| `S010` | upgrades | Security risk in contract upgrade or admin mechanisms |
| `S011` | formal_verification | Z3 proved a mathematical violation of an invariant |
| `S012` | token_interface | SEP-41 token interface compatibility or authorization deviation |

## Vulnerability Database Codes

In addition to the `S0xx` finding codes, Sanctifier ships with a community-sourced
vulnerability database (`data/vulnerability-db.json`). Matches from this database
are reported with `SOL-2024-*` identifiers (e.g. `SOL-2024-002`).

## Where codes appear

- Text output from `sanctifier analyze`
- JSON report output under:
  - `error_codes` (full mapping table)
  - each item inside `findings.*` as `code`
  - `vuln_db_matches` for vulnerability database hits
