use std::path::Path;

/// Validate a SARIF 2.1.0 JSON value against the bundled schema.
/// Returns `Ok(())` on success, or an error listing the validation failures.
pub fn validate_sarif(value: &serde_json::Value) -> anyhow::Result<()> {
    let schema_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .map(|p| p.join("schemas").join("sarif-2.1.0.json"))
        .unwrap_or_else(|| Path::new("schemas/sarif-2.1.0.json").to_path_buf());

    let schema_text = std::fs::read_to_string(&schema_path)
        .map_err(|e| anyhow::anyhow!("Cannot read SARIF schema at {}: {}", schema_path.display(), e))?;

    let schema: serde_json::Value = serde_json::from_str(&schema_text)
        .map_err(|e| anyhow::anyhow!("Invalid SARIF schema JSON: {}", e))?;

    let compiled = jsonschema::JSONSchema::compile(&schema)
        .map_err(|e| anyhow::anyhow!("Failed to compile SARIF schema: {}", e))?;

    let result = compiled.validate(value);
    if let Err(errors) = result {
        let mut msgs: Vec<String> = errors
            .map(|e| format!("  - {}: {}", e.instance_path, e))
            .collect();
        msgs.sort();
        anyhow::bail!(
            "SARIF 2.1.0 validation failed ({} error(s)):\n{}",
            msgs.len(),
            msgs.join("\n")
        );
    }

    Ok(())
}

/// Build a minimal SARIF 2.1.0 log from findings data.
pub fn build_sarif_log(
    tool_name: &str,
    tool_version: &str,
    results: Vec<serde_json::Value>,
) -> serde_json::Value {
    serde_json::json!({
        "$schema": "https://raw.githubusercontent.com/oasis-tcs/sarif-spec/master/Schemata/sarif-schema-2.1.0.json",
        "version": "2.1.0",
        "runs": [{
            "tool": {
                "driver": {
                    "name": tool_name,
                    "version": tool_version,
                    "informationUri": "https://github.com/HyperSafeD/Sanctifier",
                    "rules": []
                }
            },
            "results": results
        }]
    })
}
