use anyhow::{Context, Result};
use serde::Deserialize;
use std::{collections::HashMap, fs, path::Path};

#[derive(Debug, Clone, Deserialize)]
pub struct DetectorConfig {
    pub events_url: String,

    #[serde(default = "default_poll_interval_seconds")]
    pub poll_interval_seconds: u64,

    #[serde(default)]
    pub webhook_urls: Vec<String>,

    #[serde(default)]
    pub admins: Vec<String>,

    #[serde(default)]
    pub off_hours_windows: HashMap<String, Vec<HourWindow>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct HourWindow {
    pub start_hour: u8,
    pub end_hour: u8,
}

fn default_poll_interval_seconds() -> u64 {
    30
}

impl DetectorConfig {
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let raw = fs::read_to_string(path)
            .with_context(|| format!("failed to read detector config {}", path.display()))?;
        let config = serde_json::from_str(&raw)
            .with_context(|| format!("failed to parse detector config {}", path.display()))?;
        Ok(config)
    }
}

impl HourWindow {
    pub fn contains(&self, hour: u32) -> bool {
        let start = u32::from(self.start_hour);
        let end = u32::from(self.end_hour);
        if start <= end {
            hour >= start && hour < end
        } else {
            hour >= start || hour < end
        }
    }
}
