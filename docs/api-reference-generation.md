# API Reference Generation

Sanctifier API reference pages are generated from Rustdoc for the workspace crates. This page documents the supported behavior and the contribution notes for docs/specs changes that touch API reference generation.

## Canonical command

Use the repository `Makefile` target:

```bash
make docs
```

The target runs:

```bash
cargo doc --workspace --no-deps --open
```

For CI or headless environments, run the underlying command without opening a browser:

```bash
cargo doc --workspace --no-deps
```

Generated files are written under `target/doc/` and are not committed.

## Contribution notes

- Public Rust items that appear in generated docs should have concise doc comments when their purpose is not obvious from the item name.
- Keep examples deterministic and avoid network-dependent snippets in Rustdoc.
- If a public API is renamed or removed, update nearby user docs and examples in the same PR.
- Run `npm run docs:specs:check` when changing this page or its index links.
- Run `cargo doc --workspace --no-deps` when changing public Rust APIs or Rustdoc examples.

## Output stability

API reference generation does not change Sanctifier runtime outputs. If a future public API change also changes CLI, SARIF, JSON, schema, or contract output, document the migration in the nearest canonical format guide and include the required version bump.
