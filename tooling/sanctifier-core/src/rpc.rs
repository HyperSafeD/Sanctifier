//! RPC Provider Failover Client module for Soroban network interactions.

use serde::{Deserialize, Serialize};
use std::time::Duration;
use thiserror::Error;

/// Configuration for a single RPC provider endpoint.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RpcProviderConfig {
    /// RPC Endpoint URL (e.g. "https://soroban-testnet.stellar.org")
    pub url: String,
    /// Priority order (lower numerical value = higher priority, e.g. 1 is primary, 2 is secondary)
    pub priority: usize,
    /// Request timeout in milliseconds for this provider
    pub timeout_ms: u64,
}

/// Structured response returned by the RPC failover client.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RpcResponse {
    /// URL of the provider that successfully fulfilled the request
    pub provider_url: String,
    /// Response payload string (JSON or XDR)
    pub payload: String,
    /// HTTP status code (typically 200)
    pub status_code: u16,
    /// Whether fallback to a secondary provider occurred
    pub was_fallback: bool,
}

/// Errors returned by the RPC failover client.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum RpcError {
    /// All configured RPC providers failed or were unreachable.
    #[error("All RPC providers failed or were unreachable: {details}")]
    AllProvidersFailed { details: String },

    /// An RPC request timed out after the specified duration.
    #[error("RPC request to {url} timed out after {timeout_ms}ms")]
    Timeout { url: String, timeout_ms: u64 },

    /// An RPC provider returned a malformed response.
    #[error("Malformed RPC response from {url}: {reason}")]
    MalformedResponse { url: String, reason: String },

    /// An RPC provider returned an HTTP error status.
    #[error("HTTP error {status_code} from {url}: {message}")]
    HttpError { url: String, status_code: u16, message: String },
}

/// Resilient RPC client with automatic failover across multiple providers.
#[derive(Debug, Clone)]
pub struct RpcFailoverClient {
    /// List of configured RPC providers, sorted by priority
    pub providers: Vec<RpcProviderConfig>,
    /// Global default timeout in milliseconds
    pub default_timeout_ms: u64,
}

impl RpcFailoverClient {
    /// Create a new RPC failover client with ordered providers.
    pub fn new(mut providers: Vec<RpcProviderConfig>, default_timeout_ms: u64) -> Self {
        providers.sort_by_key(|p| p.priority);
        Self {
            providers,
            default_timeout_ms,
        }
    }

    /// Execute an RPC query with automatic failover, timeout protection, and JSON parsing validation.
    pub fn query(&self, request_payload: &str) -> Result<RpcResponse, RpcError> {
        if self.providers.is_empty() {
            return Err(RpcError::AllProvidersFailed {
                details: "No RPC providers configured".to_string(),
            });
        }

        let mut error_log = Vec::new();

        for (idx, provider) in self.providers.iter().enumerate() {
            let was_fallback = idx > 0;
            let timeout_dur = Duration::from_millis(
                if provider.timeout_ms > 0 {
                    provider.timeout_ms
                } else {
                    self.default_timeout_ms
                },
            );

            let client = match reqwest::blocking::Client::builder()
                .timeout(timeout_dur)
                .build()
            {
                Ok(c) => c,
                Err(e) => {
                    error_log.push(format!("{}: Client build error: {}", provider.url, e));
                    continue;
                }
            };

            let res = client
                .post(&provider.url)
                .header("Content-Type", "application/json")
                .body(request_payload.to_string())
                .send();

            match res {
                Ok(response) => {
                    let status = response.status().as_u16();
                    if !response.status().is_success() {
                        let msg = response.text().unwrap_or_else(|_| "Unknown HTTP error".to_string());
                        error_log.push(format!("{}: HTTP status {}", provider.url, status));
                        continue;
                    }

                    let text_body = match response.text() {
                        Ok(t) => t,
                        Err(e) => {
                            error_log.push(format!("{}: Read error: {}", provider.url, e));
                            continue;
                        }
                    };

                    // Validate JSON payload
                    let json_val: serde_json::Result<serde_json::Value> = serde_json::from_str(&text_body);
                    match json_val {
                        Ok(val) => {
                            // Check if payload contains JSON-RPC error field
                            if let Some(err_obj) = val.get("error") {
                                error_log.push(format!("{}: JSON-RPC Error: {}", provider.url, err_obj));
                                continue;
                            }

                            return Ok(RpcResponse {
                                provider_url: provider.url.clone(),
                                payload: text_body,
                                status_code: status,
                                was_fallback,
                            });
                        }
                        Err(err) => {
                            error_log.push(format!("{}: Malformed JSON syntax: {}", provider.url, err));
                            continue;
                        }
                    }
                }
                Err(err) => {
                    if err.is_timeout() {
                        error_log.push(format!("{}: Timeout after {}ms", provider.url, timeout_dur.as_millis()));
                    } else {
                        error_log.push(format!("{}: Connection failure ({})", provider.url, err));
                    }
                }
            }
        }

        Err(RpcError::AllProvidersFailed {
            details: error_log.join(" | "),
        })
    }
}
