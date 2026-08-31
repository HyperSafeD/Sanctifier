//! Chaos-testing harness for RPC provider failover resilience in sanctifier-core.
//! Exercises fault injection scenarios: full outage, partial outage failover, slow response timeout, and malformed responses.

use sanctifier_core::rpc::{RpcError, RpcFailoverClient, RpcProviderConfig};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;
use std::time::Duration;

/// Helper function to spawn a local mock HTTP server on a random available port.
fn spawn_mock_server<F>(handler: F) -> String
where
    F: Fn(&str) -> String + Send + 'static,
{
    let listener = TcpListener::bind("127.0.0.1:0").expect("failed to bind mock server listener");
    let addr = listener.local_addr().expect("failed to get local addr");

    thread::spawn(move || {
        for stream in listener.incoming() {
            if let Ok(mut stream) = stream {
                let mut buffer = [0u8; 1024];
                let _ = stream.read(&mut buffer);
                let req_text = String::from_utf8_lossy(&buffer);

                let response_str = handler(&req_text);
                let _ = stream.write_all(response_str.as_bytes());
                let _ = stream.flush();
            }
        }
    });

    format!("http://{}", addr)
}

#[test]
fn test_chaos_scenario_1_full_outage() {
    // Scenario 1: All RPC endpoints fail (HTTP 500 error / unreachable)
    let url_primary = spawn_mock_server(|_| {
        "HTTP/1.1 500 Internal Server Error\r\nContent-Type: text/plain\r\nContent-Length: 21\r\n\r\nPrimary Outage 500 Error".to_string()
    });
    let url_secondary = spawn_mock_server(|_| {
        "HTTP/1.1 503 Service Unavailable\r\nContent-Type: text/plain\r\nContent-Length: 23\r\n\r\nSecondary Outage 503 Err".to_string()
    });

    let providers = vec![
        RpcProviderConfig {
            url: url_primary,
            priority: 1,
            timeout_ms: 1000,
        },
        RpcProviderConfig {
            url: url_secondary,
            priority: 2,
            timeout_ms: 1000,
        },
    ];

    let client = RpcFailoverClient::new(providers, 1000);
    let result = client.query(r#"{"jsonrpc":"2.0","method":"getHealth","id":1}"#);

    assert!(result.is_err(), "Expected error on full RPC outage");
    if let Err(RpcError::AllProvidersFailed { details }) = result {
        assert!(
            details.contains("HTTP status 500") || details.contains("HTTP status 503"),
            "Expected outage HTTP status details, got: {}",
            details
        );
    } else {
        panic!("Unexpected error type returned: {:?}", result);
    }
}

#[test]
fn test_chaos_scenario_2_partial_outage_failover() {
    // Scenario 2: Primary RPC endpoint returns 503, secondary RPC endpoint succeeds (200 OK)
    let url_primary = spawn_mock_server(|_| {
        "HTTP/1.1 503 Service Unavailable\r\nContent-Type: text/plain\r\nContent-Length: 17\r\n\r\nNode Overloaded 503".to_string()
    });
    let url_secondary = spawn_mock_server(|_| {
        let body = r#"{"jsonrpc":"2.0","result":{"status":"healthy"},"id":1}"#;
        format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
            body.len(),
            body
        )
    });

    let providers = vec![
        RpcProviderConfig {
            url: url_primary,
            priority: 1,
            timeout_ms: 1000,
        },
        RpcProviderConfig {
            url: url_secondary,
            priority: 2,
            timeout_ms: 1000,
        },
    ];

    let client = RpcFailoverClient::new(providers, 1000);
    let result = client.query(r#"{"jsonrpc":"2.0","method":"getHealth","id":1}"#);

    assert!(result.is_ok(), "Expected transparent failover to secondary provider");
    let response = result.unwrap();
    assert!(response.was_fallback, "Should be flagged as fallback");
    assert_eq!(response.status_code, 200);
    assert!(response.payload.contains("healthy"));
}

#[test]
fn test_chaos_scenario_3_slow_response_timeout() {
    // Scenario 3: Primary RPC endpoint delays response beyond timeout threshold
    let url_primary = spawn_mock_server(|_| {
        thread::sleep(Duration::from_millis(800)); // Intentionally delay
        "HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\n{}".to_string()
    });
    let url_secondary = spawn_mock_server(|_| {
        let body = r#"{"jsonrpc":"2.0","result":{"status":"healthy"},"id":1}"#;
        format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
            body.len(),
            body
        )
    });

    let providers = vec![
        RpcProviderConfig {
            url: url_primary,
            priority: 1,
            timeout_ms: 200, // Low timeout threshold to trigger timeout
        },
        RpcProviderConfig {
            url: url_secondary,
            priority: 2,
            timeout_ms: 1000,
        },
    ];

    let client = RpcFailoverClient::new(providers, 1000);
    let start = std::time::Instant::now();
    let result = client.query(r#"{"jsonrpc":"2.0","method":"getHealth","id":1}"#);
    let elapsed = start.elapsed();

    assert!(result.is_ok(), "Expected failover after primary timeout");
    let response = result.unwrap();
    assert!(response.was_fallback, "Should fallback to secondary provider after primary timeout");
    assert!(elapsed < Duration::from_millis(1500), "Request completed without hanging indefinitely");
}

#[test]
fn test_chaos_scenario_4_malformed_response() {
    // Scenario 4: Primary RPC endpoint returns corrupted / truncated JSON syntax
    let url_primary = spawn_mock_server(|_| {
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 15\r\n\r\n{\"result\": TRUNC".to_string()
    });
    let url_secondary = spawn_mock_server(|_| {
        let body = r#"{"jsonrpc":"2.0","result":{"status":"healthy"},"id":1}"#;
        format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
            body.len(),
            body
        )
    });

    let providers = vec![
        RpcProviderConfig {
            url: url_primary,
            priority: 1,
            timeout_ms: 1000,
        },
        RpcProviderConfig {
            url: url_secondary,
            priority: 2,
            timeout_ms: 1000,
        },
    ];

    let client = RpcFailoverClient::new(providers, 1000);
    let result = client.query(r#"{"jsonrpc":"2.0","method":"getHealth","id":1}"#);

    assert!(result.is_ok(), "Expected failover when primary returns malformed JSON");
    let response = result.unwrap();
    assert!(response.was_fallback, "Should fallback to secondary when primary JSON is corrupted");
    assert!(response.payload.contains("healthy"));
}
