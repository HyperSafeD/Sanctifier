use crate::rules::{Alert, AlertSeverity};
use anyhow::Result;
use serde::Serialize;
use tracing::warn;

#[derive(Debug, Clone, Serialize)]
struct AlertWebhookPayload {
    event: &'static str,
    generated_at_unix: String,
    summary: AlertWebhookSummary,
    alerts: Vec<Alert>,
}

#[derive(Debug, Clone, Serialize)]
struct AlertWebhookSummary {
    total_alerts: usize,
    critical_alerts: usize,
    high_alerts: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WebhookProvider {
    Discord,
    Slack,
    Teams,
    Custom,
}

pub fn send_alert_webhooks(urls: &[String], alerts: &[Alert]) -> Result<()> {
    if urls.is_empty() || alerts.is_empty() {
        return Ok(());
    }

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()?;

    let payload = build_payload(alerts);
    for url in urls {
        let body = provider_payload(url, &payload);
        match client.post(url).json(&body).send() {
            Ok(resp) if resp.status().is_success() => {}
            Ok(resp) => warn!(
                target: "sanctifier-detector",
                status = resp.status().as_u16(),
                url = %url,
                "Webhook delivery failed"
            ),
            Err(err) => {
                warn!(target: "sanctifier-detector", error = %err, url = %url, "Webhook delivery error")
            }
        }
    }

    Ok(())
}

fn build_payload(alerts: &[Alert]) -> AlertWebhookPayload {
    let critical_alerts = alerts
        .iter()
        .filter(|alert| alert.severity == AlertSeverity::Critical)
        .count();
    let high_alerts = alerts
        .iter()
        .filter(|alert| alert.severity == AlertSeverity::High)
        .count();

    AlertWebhookPayload {
        event: "anomaly.alert",
        generated_at_unix: chrono::Utc::now().timestamp().to_string(),
        summary: AlertWebhookSummary {
            total_alerts: alerts.len(),
            critical_alerts,
            high_alerts,
        },
        alerts: alerts.to_vec(),
    }
}

fn provider_payload(url: &str, payload: &AlertWebhookPayload) -> serde_json::Value {
    match classify_provider(url) {
        WebhookProvider::Discord => serde_json::json!({
            "content": summary_text(payload),
        }),
        WebhookProvider::Slack => serde_json::json!({
            "text": summary_text(payload),
            "attachments": [
                {
                    "color": slack_color(payload),
                    "fields": payload.alerts.iter().map(|alert| serde_json::json!({
                        "title": format!("{} / {}", alert.contract_id, alert.rule),
                        "value": format!("{} - {}", alert.function, alert.summary),
                        "short": false
                    })).collect::<Vec<_>>()
                }
            ]
        }),
        WebhookProvider::Teams => serde_json::json!({
            "text": summary_text(payload),
        }),
        WebhookProvider::Custom => serde_json::json!(payload),
    }
}

fn summary_text(payload: &AlertWebhookPayload) -> String {
    let mut message = format!(
        "Sanctifier detector raised {} alert(s) at {}.",
        payload.summary.total_alerts, payload.generated_at_unix
    );
    if payload.summary.critical_alerts > 0 {
        message.push_str(&format!(" Critical: {}.", payload.summary.critical_alerts));
    }
    if payload.summary.high_alerts > 0 {
        message.push_str(&format!(" High: {}.", payload.summary.high_alerts));
    }
    if let Some(first) = payload.alerts.first() {
        message.push_str(&format!(
            " First alert: {} on {}. {}",
            first.rule, first.contract_id, first.summary
        ));
    }
    message
}

fn slack_color(payload: &AlertWebhookPayload) -> &'static str {
    if payload.summary.critical_alerts > 0 {
        "#d92d20"
    } else if payload.summary.high_alerts > 0 {
        "#f79009"
    } else {
        "#17b26a"
    }
}

fn classify_provider(url: &str) -> WebhookProvider {
    if url.contains("discord") {
        WebhookProvider::Discord
    } else if url.contains("slack") {
        WebhookProvider::Slack
    } else if url.contains("teams") || url.contains("office.com/webhook") {
        WebhookProvider::Teams
    } else {
        WebhookProvider::Custom
    }
}
