use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct ScanWebhookSummary {
    pub total_findings: usize,
    pub has_critical: bool,
    pub has_high: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ScanWebhookPayload {
    pub event: &'static str,
    pub project_path: String,
    pub timestamp_unix: String,
    pub summary: ScanWebhookSummary,
}

pub fn send_scan_completed_webhooks(
    urls: &[String],
    payload: &ScanWebhookPayload,
) -> anyhow::Result<()> {
    if urls.is_empty() {
        return Ok(());
    }

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()?;

    for url in urls {
        let body = provider_payload(url, payload);
        let response = client.post(url).json(&body).send();
        match response {
            Ok(resp) if resp.status().is_success() => {}
            Ok(resp) => {
                eprintln!(
                    "⚠️ Webhook delivery failed ({}): {}",
                    resp.status().as_u16(),
                    url
                );
            }
            Err(err) => {
                eprintln!("⚠️ Webhook delivery error: {} ({})", err, url);
            }
        }
    }

    Ok(())
}

fn provider_payload(url: &str, payload: &ScanWebhookPayload) -> serde_json::Value {
    let summary_text = format!(
        "Sanctifier scan completed for `{}`. Findings: {}, critical: {}, high: {}",
        payload.project_path,
        payload.summary.total_findings,
        payload.summary.has_critical,
        payload.summary.has_high
    );

    if is_discord(url) {
        serde_json::json!({ "content": summary_text })
    } else if is_slack(url) || is_teams(url) {
        serde_json::json!({ "text": summary_text })
    } else {
        serde_json::json!(payload)
    }
}

fn is_discord(url: &str) -> bool {
    url.contains("discord.com/api/webhooks") || url.contains("discordapp.com/api/webhooks")
}

fn is_slack(url: &str) -> bool {
    url.contains("hooks.slack.com")
}

fn is_teams(url: &str) -> bool {
    url.contains("outlook.office.com/webhook")
        || url.contains("office.com/webhook")
        || url.contains("webhook.office.com")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_payload() -> ScanWebhookPayload {
        ScanWebhookPayload {
            event: "scan.completed",
            project_path: "contracts/my-token".to_string(),
            timestamp_unix: "123".to_string(),
            summary: ScanWebhookSummary {
                total_findings: 2,
                has_critical: false,
                has_high: true,
            },
        }
    }

    #[test]
    fn discord_payload_uses_content() {
        let payload = provider_payload("https://discord.com/api/webhooks/1/abc", &sample_payload());
        assert!(payload.get("content").is_some());
    }

    #[test]
    fn slack_payload_uses_text() {
        let payload = provider_payload("https://hooks.slack.com/services/a/b/c", &sample_payload());
        assert!(payload.get("text").is_some());
    }

    #[test]
    fn unknown_payload_uses_struct() {
        let payload = provider_payload("https://example.com/webhook", &sample_payload());
        assert_eq!(payload["event"], "scan.completed");
        assert_eq!(payload["summary"]["total_findings"], 2);
    }
}
