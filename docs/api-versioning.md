# API Versioning and Deprecation Policy

This page describes how the public Sanctifier HTTP API is versioned and how a route is deprecated once a newer version exists. It applies to routes under `frontend/app/api/v1/`, such as `POST /api/v1/analyze`, which third-party integrators (including the browser extension) call directly.

## Current state

- Only `v1` exists. There is no `v2` yet, and no breaking change is planned.
- A breaking change is any change that removes or renames a route, parameter or response field, or changes the meaning of an existing value. Adding optional fields or new routes is not breaking.

## Rules for v1

1. Non-breaking changes ship in place under `v1`.
2. A breaking change requires a new version path (for example `frontend/app/api/v2/`). `v1` is not modified in a breaking way.
3. Once a successor exists, `v1` is marked deprecated before it is removed.

## Deprecation process

When a route is deprecated:

1. **Announce**: note the deprecation in the changelog and in the route's reference docs, and link the migration guide.
2. **Signal**: return the standard `Deprecation` and `Sunset` response headers ([RFC 9745](https://www.rfc-editor.org/rfc/rfc9745) and [RFC 8594](https://www.rfc-editor.org/rfc/rfc8594)) on every deprecated response, with a `Link` header pointing to the migration guide.
3. **Migrate**: publish a migration doc under `docs/` that maps each `v1` parameter and response field to its `v2` equivalent, with before/after examples.
4. **Sunset**: remove the deprecated route no earlier than the `Sunset` date. The minimum notice period is 6 months.

> The notice period and header requirements are the proposed policy. Maintainers should confirm them before the first deprecation.

## Checklist for a new major version

- [ ] New route tree added alongside `v1` (no in-place breaking edits to `v1`).
- [ ] Migration guide in `docs/` linked from the changelog and the route reference.
- [ ] `Deprecation` and `Sunset` headers set on `v1` responses.
- [ ] Sunset date published at least 6 months ahead.
