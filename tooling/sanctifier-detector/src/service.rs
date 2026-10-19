use crate::{
    config::DetectorConfig,
    events::{CallRecord, EventFeed},
    rules::{
        Alert, DetectionRule, FailureRateSpikeRule, FunctionSpamRule, OffHoursRule,
        PrivilegedCallRule,
    },
    webhook::send_alert_webhooks,
};
use anyhow::{Context, Result};
use reqwest::Url;
use std::{collections::HashSet, thread, time::Duration};
use tracing::info;

pub struct DetectorService {
    config: DetectorConfig,
    client: reqwest::blocking::Client,
    history: Vec<CallRecord>,
    seen_record_keys: HashSet<String>,
    seen_alert_keys: HashSet<String>,
    rules: Vec<Box<dyn DetectionRule>>,
    last_timestamp: Option<i64>,
}

impl DetectorService {
    pub fn new(config: DetectorConfig) -> Result<Self> {
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()?;

        let rules: Vec<Box<dyn DetectionRule>> = vec![
            Box::new(FailureRateSpikeRule::default()),
            Box::new(FunctionSpamRule::default()),
            Box::new(PrivilegedCallRule::new(config.admins.clone())),
            Box::new(OffHoursRule::new(config.off_hours_windows.clone())),
        ];

        Ok(Self {
            config,
            client,
            history: Vec::new(),
            seen_record_keys: HashSet::new(),
            seen_alert_keys: HashSet::new(),
            rules,
            last_timestamp: None,
        })
    }

    pub fn run(&mut self) -> Result<()> {
        loop {
            self.poll_once()?;
            thread::sleep(Duration::from_secs(
                self.config.poll_interval_seconds.max(1),
            ));
        }
    }

    pub fn poll_once(&mut self) -> Result<Vec<Alert>> {
        let new_records = self.fetch_records()?;
        let mut ingested = 0usize;
        for record in new_records {
            let key = record.stable_key();
            if self.seen_record_keys.insert(key) {
                self.last_timestamp = Some(
                    self.last_timestamp
                        .map_or(record.timestamp_unix, |ts| ts.max(record.timestamp_unix)),
                );
                self.history.push(record);
                ingested += 1;
            }
        }

        if ingested == 0 && self.history.is_empty() {
            return Ok(Vec::new());
        }

        if self.history.len() > 10_000 {
            let drain_count = self.history.len() - 10_000;
            self.history.drain(0..drain_count);
        }

        let mut alerts = Vec::new();
        for rule in &self.rules {
            alerts.extend(rule.evaluate(&self.history));
        }

        let fresh_alerts: Vec<_> = alerts
            .into_iter()
            .filter(|alert| self.seen_alert_keys.insert(alert.fingerprint.clone()))
            .collect();

        if !fresh_alerts.is_empty() {
            info!(alerts = fresh_alerts.len(), "dispatching anomaly alerts");
            send_alert_webhooks(&self.config.webhook_urls, &fresh_alerts)
                .context("failed to deliver anomaly webhooks")?;
        }

        Ok(fresh_alerts)
    }

    fn fetch_records(&self) -> Result<Vec<CallRecord>> {
        let mut url = Url::parse(&self.config.events_url)
            .with_context(|| format!("invalid events url {}", self.config.events_url))?;
        if let Some(last_timestamp) = self.last_timestamp {
            url.query_pairs_mut()
                .append_pair("since", &last_timestamp.to_string());
        }

        let response = self.client.get(url).send()?.error_for_status()?;
        let feed = response.json::<EventFeed>()?;
        Ok(match feed {
            EventFeed::Records(records) => records,
            EventFeed::Envelope { records } => records,
        })
    }
}
