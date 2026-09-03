use crate::{config::HourWindow, events::CallRecord};
use chrono::{TimeZone, Utc};
use serde::Serialize;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum AlertSeverity {
    Critical,
    High,
    Medium,
}

#[derive(Debug, Clone, Serialize)]
pub struct Alert {
    pub fingerprint: String,
    pub rule: String,
    pub severity: AlertSeverity,
    pub contract_id: String,
    pub caller: String,
    pub function: String,
    pub timestamp_unix: i64,
    pub summary: String,
}

pub trait DetectionRule: Send + Sync {
    fn name(&self) -> &'static str;
    fn evaluate(&self, records: &[CallRecord]) -> Vec<Alert>;
}

#[derive(Debug, Clone)]
pub struct FailureRateSpikeRule {
    recent_window: usize,
    baseline_window: usize,
}

impl Default for FailureRateSpikeRule {
    fn default() -> Self {
        Self {
            recent_window: 50,
            baseline_window: 200,
        }
    }
}

impl DetectionRule for FailureRateSpikeRule {
    fn name(&self) -> &'static str {
        "failure-rate-spike"
    }

    fn evaluate(&self, records: &[CallRecord]) -> Vec<Alert> {
        let mut alerts = Vec::new();
        let mut by_contract: HashMap<&str, Vec<&CallRecord>> = HashMap::new();
        for record in records {
            by_contract
                .entry(&record.contract_id)
                .or_default()
                .push(record);
        }

        for (contract_id, mut history) in by_contract {
            history.sort_by_key(|record| record.timestamp_unix);
            if history.len() < self.recent_window + self.baseline_window {
                continue;
            }

            let recent = &history[history.len() - self.recent_window..];
            let baseline = &history[history.len() - self.recent_window - self.baseline_window
                ..history.len() - self.recent_window];

            let recent_failures = recent.iter().filter(|record| !record.success).count() as f64;
            let baseline_failures = baseline.iter().filter(|record| !record.success).count() as f64;
            let recent_rate = recent_failures / recent.len() as f64;
            let baseline_rate = baseline_failures / baseline.len() as f64;

            if baseline_rate < 0.05 && recent_rate > 0.20 {
                let latest = recent.last().expect("recent window is non-empty");
                alerts.push(Alert {
                    fingerprint: format!(
                        "{}:{}:{}",
                        self.name(),
                        contract_id,
                        latest.timestamp_unix
                    ),
                    rule: self.name().to_string(),
                    severity: AlertSeverity::Critical,
                    contract_id: contract_id.to_string(),
                    caller: latest.caller.clone(),
                    function: latest.function.clone(),
                    timestamp_unix: latest.timestamp_unix,
                    summary: format!(
                        "failure rate jumped to {:.0}% over the last {} calls (baseline {:.0}% over the previous {} calls)",
                        recent_rate * 100.0,
                        self.recent_window,
                        baseline_rate * 100.0,
                        self.baseline_window,
                    ),
                });
            }
        }

        alerts
    }
}

#[derive(Debug, Clone)]
pub struct FunctionSpamRule {
    threshold: usize,
    window_seconds: i64,
}

impl Default for FunctionSpamRule {
    fn default() -> Self {
        Self {
            threshold: 100,
            window_seconds: 60 * 60,
        }
    }
}

impl DetectionRule for FunctionSpamRule {
    fn name(&self) -> &'static str {
        "function-spam"
    }

    fn evaluate(&self, records: &[CallRecord]) -> Vec<Alert> {
        let mut alerts = Vec::new();
        let Some(latest_ts) = records.iter().map(|record| record.timestamp_unix).max() else {
            return alerts;
        };
        let cutoff = latest_ts - self.window_seconds;
        let mut counts: HashMap<(String, String, String), usize> = HashMap::new();

        for record in records
            .iter()
            .filter(|record| record.timestamp_unix >= cutoff)
        {
            let key = (
                record.contract_id.clone(),
                record.caller.clone(),
                record.function.clone(),
            );
            *counts.entry(key).or_default() += 1;
        }

        for ((contract_id, caller, function), count) in counts {
            if count > self.threshold {
                let summary = format!(
                    "{} invoked {} {} times in the last 60 minutes",
                    caller, function, count
                );
                alerts.push(Alert {
                    fingerprint: format!(
                        "{}:{}:{}:{}:{}",
                        self.name(),
                        contract_id,
                        caller,
                        function,
                        latest_ts
                    ),
                    rule: self.name().to_string(),
                    severity: AlertSeverity::High,
                    contract_id,
                    caller,
                    function,
                    timestamp_unix: latest_ts,
                    summary,
                });
            }
        }

        alerts
    }
}

#[derive(Debug, Clone)]
pub struct PrivilegedCallRule {
    admins: HashSet<String>,
}

impl PrivilegedCallRule {
    pub fn new(admins: impl IntoIterator<Item = String>) -> Self {
        Self {
            admins: admins.into_iter().collect(),
        }
    }
}

impl DetectionRule for PrivilegedCallRule {
    fn name(&self) -> &'static str {
        "privileged-call"
    }

    fn evaluate(&self, records: &[CallRecord]) -> Vec<Alert> {
        let mut alerts = Vec::new();
        for record in records {
            if !matches!(record.function.as_str(), "pause" | "resume") {
                continue;
            }
            if self.admins.contains(&record.caller) {
                continue;
            }

            alerts.push(Alert {
                fingerprint: format!(
                    "{}:{}:{}:{}",
                    self.name(),
                    record.contract_id,
                    record.function,
                    record.stable_key()
                ),
                rule: self.name().to_string(),
                severity: AlertSeverity::Critical,
                contract_id: record.contract_id.clone(),
                caller: record.caller.clone(),
                function: record.function.clone(),
                timestamp_unix: record.timestamp_unix,
                summary: format!(
                    "privileged {} call from non-admin address {}",
                    record.function, record.caller
                ),
            });
        }
        alerts
    }
}

#[derive(Debug, Clone)]
pub struct OffHoursRule {
    windows_by_contract: HashMap<String, Vec<HourWindow>>,
}

impl OffHoursRule {
    pub fn new(windows_by_contract: HashMap<String, Vec<HourWindow>>) -> Self {
        Self {
            windows_by_contract,
        }
    }
}

impl DetectionRule for OffHoursRule {
    fn name(&self) -> &'static str {
        "off-hours-activity"
    }

    fn evaluate(&self, records: &[CallRecord]) -> Vec<Alert> {
        let mut alerts = Vec::new();

        for record in records {
            let Some(windows) = self.windows_by_contract.get(&record.contract_id) else {
                continue;
            };
            let Some(ts) = Utc.timestamp_opt(record.timestamp_unix, 0).single() else {
                continue;
            };
            let hour = ts.hour();
            if windows.iter().any(|window| window.contains(hour)) {
                continue;
            }

            alerts.push(Alert {
                fingerprint: format!(
                    "{}:{}:{}",
                    self.name(),
                    record.contract_id,
                    record.stable_key()
                ),
                rule: self.name().to_string(),
                severity: AlertSeverity::Medium,
                contract_id: record.contract_id.clone(),
                caller: record.caller.clone(),
                function: record.function.clone(),
                timestamp_unix: record.timestamp_unix,
                summary: format!(
                    "call landed outside the configured operating window at {:02}:00 UTC",
                    hour
                ),
            });
        }

        alerts
    }
}

use chrono::Timelike;
