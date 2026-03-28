use serde::Serialize;
use tracing::warn;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WebhookProvider {
    Discord,
    Slack,
    Teams,
    Custom,
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
                warn!(
                    target: "sanctifier",
                    status = resp.status().as_u16(),
                    url = %url,
                    "Webhook delivery failed"
                );
            }
            Err(err) => {
                warn!(target: "sanctifier", error = %err, url = %url, "Webhook delivery error");
            }
        }
    }

    Ok(())
}

fn provider_payload(url: &str, payload: &ScanWebhookPayload) -> serde_json::Value {
    provider_payload_for(classify_provider(url), payload)
}

fn provider_payload_for(
    provider: WebhookProvider,
    payload: &ScanWebhookPayload,
) -> serde_json::Value {
    let summary_text = summary_text(payload);

    match provider {
        WebhookProvider::Discord => serde_json::json!({
            "content": summary_text,
        }),
        WebhookProvider::Slack => serde_json::json!({
            "text": summary_text,
            "attachments": [
                {
                    "color": slack_color(payload),
                    "fields": [
                        {
                            "title": "Project",
                            "value": payload.project_path,
                            "short": true
                        },
                        {
                            "title": "Event",
                            "value": payload.event,
                            "short": true
                        },
                        {
                            "title": "Total Findings",
                            "value": payload.summary.total_findings.to_string(),
                            "short": true
                        },
                        {
                            "title": "Critical",
                            "value": payload.summary.has_critical.to_string(),
                            "short": true
                        },
                        {
                            "title": "High",
                            "value": payload.summary.has_high.to_string(),
                            "short": true
                        },
                        {
                            "title": "Timestamp",
                            "value": payload.timestamp_unix,
                            "short": true
                        }
                    ]
                }
            ]
        }),
        WebhookProvider::Teams => serde_json::json!({ "text": summary_text }),
        WebhookProvider::Custom => serde_json::json!(payload),
    }
}

fn classify_provider(url: &str) -> WebhookProvider {
    if has_provider_hint(url, "discord") || is_discord(url) {
        WebhookProvider::Discord
    } else if has_provider_hint(url, "slack") || is_slack(url) {
        WebhookProvider::Slack
    } else if has_provider_hint(url, "teams") || is_teams(url) {
        WebhookProvider::Teams
    } else {
        WebhookProvider::Custom
    }
}

fn has_provider_hint(url: &str, provider: &str) -> bool {
    url.contains(&format!("sanctifier_provider={provider}"))
}

fn summary_text(payload: &ScanWebhookPayload) -> String {
    format!(
        "Sanctifier scan completed for `{}`. Findings: {}, critical: {}, high: {}",
        payload.project_path,
        payload.summary.total_findings,
        payload.summary.has_critical,
        payload.summary.has_high
    )
}

fn slack_color(payload: &ScanWebhookPayload) -> &'static str {
    if payload.summary.has_critical {
        "#d92d20"
    } else if payload.summary.has_high {
        "#f79009"
    } else {
        "#17b26a"
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
    use mockito::{Matcher, Server};

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
    fn discord_payload_matches_expected_json_schema() {
        let payload = sample_payload();
        let expected_body = serde_json::json!({
            "content": summary_text(&payload),
        });

        let mut server = Server::new();
        let mock = server
            .mock("POST", "/discord")
            .match_query(Matcher::Any)
            .match_body(Matcher::Json(expected_body))
            .with_status(204)
            .create();

        let url = format!("{}/discord?sanctifier_provider=discord", server.url());
        send_scan_completed_webhooks(&[url], &payload).unwrap();

        mock.assert();
    }

    #[test]
    fn slack_payload_matches_expected_json_schema() {
        let payload = sample_payload();
        let expected_body = serde_json::json!({
            "text": summary_text(&payload),
            "attachments": [
                {
                    "color": "#f79009",
                    "fields": [
                        {
                            "title": "Project",
                            "value": "contracts/my-token",
                            "short": true
                        },
                        {
                            "title": "Event",
                            "value": "scan.completed",
                            "short": true
                        },
                        {
                            "title": "Total Findings",
                            "value": "2",
                            "short": true
                        },
                        {
                            "title": "Critical",
                            "value": "false",
                            "short": true
                        },
                        {
                            "title": "High",
                            "value": "true",
                            "short": true
                        },
                        {
                            "title": "Timestamp",
                            "value": "123",
                            "short": true
                        }
                    ]
                }
            ]
        });

        let mut server = Server::new();
        let mock = server
            .mock("POST", "/slack")
            .match_query(Matcher::Any)
            .match_body(Matcher::Json(expected_body))
            .with_status(200)
            .create();

        let url = format!("{}/slack?sanctifier_provider=slack", server.url());
        send_scan_completed_webhooks(&[url], &payload).unwrap();

        mock.assert();
    }

    #[test]
    fn multiple_webhook_urls_all_receive_notification() {
        let mut first = Server::new();
        let mut second = Server::new();

        let first_mock = first.mock("POST", "/notify").with_status(200).create();
        let second_mock = second.mock("POST", "/notify").with_status(200).create();

        let urls = vec![
            format!("{}/notify", first.url()),
            format!("{}/notify", second.url()),
        ];

        send_scan_completed_webhooks(&urls, &sample_payload()).unwrap();

        first_mock.assert();
        second_mock.assert();
    }

    #[test]
    fn unknown_payload_uses_struct() {
        let payload = provider_payload("https://example.com/webhook", &sample_payload());
        assert_eq!(payload["event"], "scan.completed");
        assert_eq!(payload["summary"]["total_findings"], 2);
    }
}
