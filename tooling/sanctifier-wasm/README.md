# sanctifier-wasm

WebAssembly bindings for Sanctifier analysis.

## Exported API

- `analyze(source)`
- `analyze_with_config(config_json, source)`
- `analyze_with_progress(source)`
- `finding_codes()`
- `default_config_json()`
- `version()`
- `schema_version()`
- `asset_cache_key()`
- `cache_metadata()`

## Offline caching integration

Use `asset_cache_key()` or `cache_metadata().cache_key` when storing wasm assets in CacheStorage or a service worker. The key changes when either package version or schema version changes, so stale assets are safely evicted. This improves release and publishing reliability by providing predictable outputs for frontend applications.

## Web Worker / Parallelization Strategy

To avoid blocking the main UI thread during intensive static analysis, it is recommended to run the `@sanctifier/wasm` module inside a Web Worker. See `examples/web_worker.js` for an implementation reference. By delegating analysis requests to background workers, you can ensure a smooth user experience even on large codebases.

## API Surface Stability for Frontend

The `@sanctifier/wasm` package maintains a strict API surface stability contract for frontend consumers. 
- All breaking changes to the exported functions will result in a major version bump.
- The `schema_version()` function returns the data shape version of the analysis output.
- For stable integrations, always check the `schema_version()` or `version()` to implement safe fallback behaviors in the frontend.
