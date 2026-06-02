use serde::Serialize;
use std::env;
use std::time::Duration;

#[derive(Debug, Clone, Serialize)]
pub struct AnalysisTelemetry {
    pub tool_version: String,
    pub duration_ms: u64,
    pub rule_ids: Vec<String>,
}

pub fn sanitize_version(version: &str) -> String {
    version
        .split_once('+')
        .map(|(core, _)| core)
        .unwrap_or(version)
        .split_once('-')
        .map(|(core, _)| core)
        .unwrap_or(version)
        .to_string()
}

pub fn emit_analysis_telemetry(payload: &AnalysisTelemetry) -> anyhow::Result<()> {
    let endpoint = match env::var("SANCTIFIER_TELEMETRY_URL") {
        Ok(value) if !value.trim().is_empty() => value,
        _ => return Ok(()),
    };

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()?;

    let response = client.post(endpoint).json(payload).send()?;
    if response.status().is_success() {
        Ok(())
    } else {
        Err(anyhow::anyhow!(
            "telemetry endpoint returned HTTP {}",
            response.status()
        ))
    }
}
