#![allow(dead_code)]

// #522 — Security hardening + threat model notes for webhook delivery.
//
// Threat model:
//   T1 – Spoofed payloads: an attacker sends a crafted POST to the same endpoint.
//        Mitigation: HMAC-SHA256 signature in `X-Sanctifier-Signature-256` header.
//   T2 – Transient network failures cause missed notifications.
//        Mitigation: exponential-backoff retry (3 attempts, base 1 s, cap 30 s).
//   T3 – Slow or unresponsive endpoints block the analysis pipeline indefinitely.
//        Mitigation: per-request timeout (10 s) enforced by reqwest.
//   T4 – SSRF via attacker-controlled webhook URL (if URL is user-supplied config).
//        Mitigation: URL scheme must be https (enforced by `validate_webhook_url`);
//        private-range IPs are not blocked here — operators should apply egress rules.
//   T5 – Secret leakage in logs.
//        Mitigation: secret is never logged; only the HMAC hex digest is transmitted.

use hmac::{Hmac, Mac};
use serde::Serialize;
use sha2::Sha256;
use tracing::{info, warn};

type HmacSha256 = Hmac<Sha256>;

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

/// Configuration for webhook delivery (#522).
#[derive(Debug, Clone, Default)]
pub struct WebhookConfig {
    /// If set, every outgoing request includes an `X-Sanctifier-Signature-256`
    /// header containing `sha256=<hmac-hex>` computed over the serialised body.
    pub secret: Option<String>,
    /// Maximum number of delivery attempts per URL (default: 3).
    pub max_attempts: Option<u32>,
}

/// Validate that a webhook URL uses HTTPS (T4 mitigation).
pub fn validate_webhook_url(url: &str) -> Result<(), String> {
    if url.starts_with("https://") {
        Ok(())
    } else {
        Err(format!(
            "webhook URL '{}' must use HTTPS to prevent plaintext secret transmission",
            url
        ))
    }
}

/// Compute an HMAC-SHA256 signature over `body` using `secret`.
/// Returns the hex-encoded digest prefixed with `sha256=`.
fn hmac_signature(secret: &str, body: &[u8]) -> String {
    let mut mac =
        HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC accepts any key length");
    mac.update(body);
    let result = mac.finalize();
    let bytes = result.into_bytes();
    format!("sha256={}", hex_encode(&bytes))
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

/// Deliver webhooks with optional HMAC signing and exponential-backoff retries.
///
/// Errors are accumulated and returned after all URLs are attempted, so a
/// single failing endpoint does not abort delivery to the remaining ones.
pub fn send_scan_completed_webhooks(
    urls: &[String],
    payload: &ScanWebhookPayload,
    config: &WebhookConfig,
) -> anyhow::Result<()> {
    if urls.is_empty() {
        return Ok(());
    }

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()?;

    let max_attempts = config.max_attempts.unwrap_or(3);
    let mut errors: Vec<String> = Vec::new();

    for url in urls {
        let body = provider_payload(url, payload);
        let body_bytes = serde_json::to_vec(&body)?;

        let mut last_error: Option<String> = None;

        for attempt in 1..=max_attempts {
            let mut req = client
                .post(url)
                .header("Content-Type", "application/json")
                .body(body_bytes.clone());

            if let Some(ref secret) = config.secret {
                let sig = hmac_signature(secret, &body_bytes);
                req = req.header("X-Sanctifier-Signature-256", sig);
            }

            match req.send() {
                Ok(resp) if resp.status().is_success() => {
                    if attempt > 1 {
                        info!(
                            target: "sanctifier",
                            url = %url,
                            attempt,
                            "Webhook delivered after retry"
                        );
                    }
                    last_error = None;
                    break;
                }
                Ok(resp) => {
                    let status = resp.status().as_u16();
                    last_error = Some(format!("HTTP {}", status));
                    warn!(
                        target: "sanctifier",
                        status,
                        url = %url,
                        attempt,
                        max_attempts,
                        "Webhook delivery failed, will retry"
                    );
                }
                Err(err) => {
                    last_error = Some(err.to_string());
                    warn!(
                        target: "sanctifier",
                        error = %err,
                        url = %url,
                        attempt,
                        max_attempts,
                        "Webhook request error, will retry"
                    );
                }
            }

            if attempt < max_attempts {
                // Exponential backoff: 1s, 2s, 4s … capped at 30s.
                let delay_secs = std::cmp::min(1u64 << (attempt - 1), 30);
                std::thread::sleep(std::time::Duration::from_secs(delay_secs));
            }
        }

        if let Some(err_msg) = last_error {
            errors.push(format!("{url}: {err_msg}"));
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(anyhow::anyhow!(
            "webhook delivery failed for {} endpoint(s):\n{}",
            errors.len(),
            errors.join("\n")
        ))
    }
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
        send_scan_completed_webhooks(&[url], &payload, &WebhookConfig::default()).unwrap();

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
                        { "title": "Project",        "value": "contracts/my-token", "short": true },
                        { "title": "Event",          "value": "scan.completed",     "short": true },
                        { "title": "Total Findings", "value": "2",                  "short": true },
                        { "title": "Critical",       "value": "false",              "short": true },
                        { "title": "High",           "value": "true",               "short": true },
                        { "title": "Timestamp",      "value": "123",                "short": true }
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
        send_scan_completed_webhooks(&[url], &payload, &WebhookConfig::default()).unwrap();

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

        send_scan_completed_webhooks(&urls, &sample_payload(), &WebhookConfig::default()).unwrap();

        first_mock.assert();
        second_mock.assert();
    }

    #[test]
    fn unknown_payload_uses_struct() {
        let payload = provider_payload("https://example.com/webhook", &sample_payload());
        assert_eq!(payload["event"], "scan.completed");
        assert_eq!(payload["summary"]["total_findings"], 2);
    }

    #[test]
    fn signed_request_includes_hmac_header() {
        let secret = "test-secret-key";
        let payload = sample_payload();
        let body = provider_payload_for(WebhookProvider::Custom, &payload);
        let body_bytes = serde_json::to_vec(&body).unwrap();
        let expected_sig = hmac_signature(secret, &body_bytes);

        let mut server = Server::new();
        let mock = server
            .mock("POST", "/signed")
            .match_header("X-Sanctifier-Signature-256", expected_sig.as_str())
            .with_status(200)
            .create();

        let url = format!("{}/signed", server.url());
        let config = WebhookConfig {
            secret: Some(secret.to_string()),
            max_attempts: Some(1),
        };
        send_scan_completed_webhooks(&[url], &payload, &config).unwrap();
        mock.assert();
    }

    #[test]
    fn delivery_failure_after_retries_returns_error() {
        let mut server = Server::new();
        // Always return 500 — should exhaust retries
        let mock = server
            .mock("POST", "/fail")
            .with_status(500)
            .expect(1) // max_attempts = 1 to keep test fast
            .create();

        let url = format!("{}/fail", server.url());
        let config = WebhookConfig {
            secret: None,
            max_attempts: Some(1),
        };
        let result = send_scan_completed_webhooks(&[url], &sample_payload(), &config);
        assert!(result.is_err(), "should return Err after exhausted retries");
        mock.assert();
    }

    #[test]
    fn empty_url_list_returns_ok_without_requests() {
        send_scan_completed_webhooks(&[], &sample_payload(), &WebhookConfig::default()).unwrap();
    }

    #[test]
    fn validate_webhook_url_rejects_http() {
        assert!(validate_webhook_url("http://hooks.slack.com/xxx").is_err());
    }

    #[test]
    fn validate_webhook_url_accepts_https() {
        assert!(validate_webhook_url("https://hooks.slack.com/xxx").is_ok());
    }

    #[test]
    fn hmac_signature_is_deterministic() {
        let sig1 = hmac_signature("secret", b"body");
        let sig2 = hmac_signature("secret", b"body");
        assert_eq!(sig1, sig2);
        assert!(sig1.starts_with("sha256="));
    }

    #[test]
    fn hmac_signature_differs_with_different_secrets() {
        let sig1 = hmac_signature("secret-a", b"body");
        let sig2 = hmac_signature("secret-b", b"body");
        assert_ne!(sig1, sig2);
    }
}
