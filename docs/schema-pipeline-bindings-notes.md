# Schema Publish Pipeline & Typed Bindings (Threat Model)

## Schema Publish Pipeline
Schemas in `schemas/` and `data/` are validated via CI rules before being available to users. In the publish pipeline, regex compilation is verified for safe limits to prevent ReDoS payloads. Outputs are published safely.

## Typed Bindings Generation Threat Model
When generating bindings (e.g. WASM or Python scripts from schema defs), ensure the schema has trusted provenance. Do not execute unvalidated user inputs directly during parsing or bindings code emission.
