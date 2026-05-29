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

Use `asset_cache_key()` or `cache_metadata().cache_key` when storing wasm assets in CacheStorage or a service worker. The key changes when either package version or schema version changes, so stale assets are safely evicted.
