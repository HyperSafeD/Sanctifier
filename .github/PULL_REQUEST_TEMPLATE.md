## Summary

Describe the change, the motivation behind it, and any important implementation details.

Fixes #

## Type of change

- [ ] Bug fix
- [ ] New feature
- [ ] Breaking change
- [ ] Documentation update
- [ ] Maintenance or refactor

## Testing

List the commands you ran and the scope of validation.

```text
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test -p sanctifier-core --all-features
cargo test -p sanctifier-cli
cd frontend && npm test
```

## Checklist

- [ ] I ran the relevant tests locally, or explained why they were not needed.
- [ ] I updated documentation for any user-facing behavior changes.
- [ ] I added or updated tests for the change when appropriate.
- [ ] I added a changelog or release-notes entry when needed, or confirmed none is required.
- [ ] I verified this branch is up to date with `main` and merge conflicts are resolved.
