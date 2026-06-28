//! Structured error types with actionable user hints (#528).
//!
//! Every `SanctifierError` variant carries a machine-readable `code`, a
//! human-readable `message`, and an `hint` that tells the user exactly what
//! to do next. The `Display` impl renders all three so that `anyhow` context
//! propagation surfaces the hint automatically at the top level.

use std::fmt;

/// Error codes (stable; used in structured output and CI gate rules).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    /// Path supplied to a command was not found or is inaccessible.
    E001,
    /// Path exists but is not a Soroban project (no `Cargo.toml` with `soroban-sdk`).
    E002,
    /// Configuration file could not be parsed.
    E003,
    /// Analysis timed out for one or more files.
    E004,
    /// Webhook delivery failed after all retries.
    E005,
    /// `cargo search` or `cargo install` failed during self-update.
    E006,
    /// Report output path could not be written.
    E007,
    /// Vulnerability database file is missing or malformed.
    E008,
    /// Dry-run mode: no changes were made (informational, not a failure).
    E009,
    /// Deploy target network is not a recognised Soroban network handle.
    E010,
    /// Soroban credentials are missing or cannot be sourced from the environment.
    E011,
}

impl ErrorCode {
    pub fn as_str(self) -> &'static str {
        match self {
            ErrorCode::E001 => "E001",
            ErrorCode::E002 => "E002",
            ErrorCode::E003 => "E003",
            ErrorCode::E004 => "E004",
            ErrorCode::E005 => "E005",
            ErrorCode::E006 => "E006",
            ErrorCode::E007 => "E007",
            ErrorCode::E008 => "E008",
            ErrorCode::E009 => "E009",
            ErrorCode::E010 => "E010",
            ErrorCode::E011 => "E011",
        }
    }
}

/// A Sanctifier CLI error with a stable code, a message, and an actionable hint.
#[derive(Debug)]
pub struct SanctifierError {
    pub code: ErrorCode,
    pub message: String,
    pub hint: String,
}

impl SanctifierError {
    pub fn new(code: ErrorCode, message: impl Into<String>, hint: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            hint: hint.into(),
        }
    }

    // ── Constructors for each error code ─────────────────────────────────────

    pub fn path_not_found(path: &std::path::Path) -> Self {
        Self::new(
            ErrorCode::E001,
            format!("path not found: {}", path.display()),
            format!(
                "Check that '{}' exists and that you have read permission. \
                 Run `ls {}` to inspect the directory.",
                path.display(),
                path.parent()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|| ".".to_string())
            ),
        )
    }

    pub fn not_soroban_project(path: &std::path::Path) -> Self {
        Self::new(
            ErrorCode::E002,
            format!("'{}' is not a valid Soroban project", path.display()),
            "Ensure the directory contains a Cargo.toml that declares \
             `soroban-sdk` as a dependency. Run `sanctifier init` to scaffold \
             a new project, or pass a path to an existing Soroban contract."
                .to_string(),
        )
    }

    pub fn config_parse_error(path: &std::path::Path, detail: &str) -> Self {
        Self::new(
            ErrorCode::E003,
            format!("could not parse config file {}: {}", path.display(), detail),
            format!(
                "Validate '{}' against the JSON schema in `schemas/sanctifier-config.json`. \
                 Run `sanctifier doctor` to diagnose common configuration problems.",
                path.display()
            ),
        )
    }

    pub fn analysis_timeout(file: &str, timeout_secs: u64) -> Self {
        Self::new(
            ErrorCode::E004,
            format!("analysis timed out for '{}' after {}s", file, timeout_secs),
            format!(
                "Increase `--timeout` (currently {timeout_secs}s) or add the file to \
                 `ignore_paths` in `.sanctifier.toml` if it is generated/vendored code."
            ),
        )
    }

    pub fn webhook_failed(url: &str, attempts: u32, last_error: &str) -> Self {
        Self::new(
            ErrorCode::E005,
            format!(
                "webhook delivery to '{}' failed after {} attempt(s): {}",
                url, attempts, last_error
            ),
            "Verify the webhook URL is reachable from this machine and that the \
             endpoint accepts POST requests with a JSON body. Check `--webhook-secret` \
             is set if the endpoint requires HMAC-SHA256 signature verification."
                .to_string(),
        )
    }

    pub fn update_cargo_search_failed(detail: &str) -> Self {
        Self::new(
            ErrorCode::E006,
            format!("`cargo search` failed: {}", detail),
            "Ensure you are connected to the internet and that `~/.cargo/registry` \
             is not corrupted. Try running `cargo search sanctifier-cli` manually \
             to diagnose the issue."
                .to_string(),
        )
    }

    pub fn update_install_failed(version: &str) -> Self {
        Self::new(
            ErrorCode::E006,
            format!(
                "`cargo install` failed while installing sanctifier-cli v{}",
                version
            ),
            format!(
                "Try running `cargo install sanctifier-cli --version {version} --locked` \
                 manually to see the full error. If the Rust toolchain is out of date, \
                 run `rustup update stable` first."
            ),
        )
    }

    pub fn report_write_failed(path: &std::path::Path, detail: &str) -> Self {
        Self::new(
            ErrorCode::E007,
            format!("could not write report to '{}': {}", path.display(), detail),
            format!(
                "Check that the directory '{}' exists and is writable. \
                 Create it with `mkdir -p {}` if needed.",
                path.parent()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|| ".".to_string()),
                path.parent()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|| ".".to_string()),
            ),
        )
    }

    pub fn vuln_db_load_failed(path: &std::path::Path, detail: &str) -> Self {
        Self::new(
            ErrorCode::E008,
            format!(
                "could not load vulnerability database from '{}': {}",
                path.display(),
                detail
            ),
            format!(
                "Ensure '{}' is a valid JSON file matching the vuln-db schema. \
                 Run `sanctifier vulndb validate {}` to check the file, or omit \
                 `--vuln-db` to use the bundled default database.",
                path.display(),
                path.display()
            ),
        )
    }

    pub fn invalid_network(network: &str) -> Self {
        Self::new(
            ErrorCode::E010,
            format!("'{}' is not a recognised Soroban network", network),
            "Use one of: testnet, futurenet, mainnet. \
             See https://developers.stellar.org/network/soroban-rpc for RPC endpoints."
                .to_string(),
        )
    }

    pub fn missing_credentials() -> Self {
        Self::new(
            ErrorCode::E011,
            "Soroban secret key not provided",
            "Pass --secret-key <SK…> or set the SOROBAN_SECRET_KEY environment variable. \
             Generate a funded testnet key with: \
             `stellar keys generate --global my-key --network testnet && \
             stellar keys fund my-key --network testnet`."
                .to_string(),
        )
    }

    pub fn dry_run_no_changes(command: &str) -> Self {
        Self::new(
            ErrorCode::E009,
            format!(
                "[dry-run] '{}' would make changes but --dry-run is set",
                command
            ),
            "Remove `--dry-run` to apply the changes, or review what would change \
             before proceeding."
                .to_string(),
        )
    }
}

impl fmt::Display for SanctifierError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[{}] {}\n  → hint: {}",
            self.code.as_str(),
            self.message,
            self.hint
        )
    }
}

impl std::error::Error for SanctifierError {}

/// Render a `SanctifierError` as a JSON object for `--output-format json` callers.
pub fn to_json(err: &SanctifierError) -> serde_json::Value {
    serde_json::json!({
        "error": {
            "code": err.code.as_str(),
            "message": err.message,
            "hint": err.hint
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn display_includes_code_message_and_hint() {
        let err = SanctifierError::path_not_found(Path::new("/no/such/path"));
        let s = err.to_string();
        assert!(s.contains("[E001]"), "missing error code");
        assert!(s.contains("path not found"), "missing message");
        assert!(s.contains("hint:"), "missing hint marker");
    }

    #[test]
    fn not_soroban_project_error_mentions_cargo_toml() {
        let err = SanctifierError::not_soroban_project(Path::new("my-contract"));
        assert!(err.hint.contains("Cargo.toml"));
        assert!(err.hint.contains("sanctifier init"));
        assert_eq!(err.code, ErrorCode::E002);
    }

    #[test]
    fn config_parse_error_mentions_doctor_command() {
        let err = SanctifierError::config_parse_error(
            Path::new(".sanctifier.toml"),
            "unexpected key `foo`",
        );
        assert!(err.hint.contains("sanctifier doctor"));
        assert_eq!(err.code, ErrorCode::E003);
    }

    #[test]
    fn analysis_timeout_hint_includes_timeout_value() {
        let err = SanctifierError::analysis_timeout("src/lib.rs", 30);
        assert!(err.hint.contains("30s"));
        assert!(err.hint.contains("ignore_paths"));
    }

    #[test]
    fn webhook_failed_hint_mentions_secret() {
        let err =
            SanctifierError::webhook_failed("https://hooks.slack.com/xxx", 3, "connection refused");
        assert!(err.hint.contains("--webhook-secret"));
        assert_eq!(err.code, ErrorCode::E005);
    }

    #[test]
    fn update_install_failed_hint_includes_version() {
        let err = SanctifierError::update_install_failed("0.4.0");
        assert!(err.hint.contains("0.4.0"));
        assert!(err.hint.contains("rustup"));
    }

    #[test]
    fn report_write_failed_hint_includes_mkdir() {
        let err =
            SanctifierError::report_write_failed(Path::new("out/report.md"), "permission denied");
        assert!(err.hint.contains("mkdir"));
        assert_eq!(err.code, ErrorCode::E007);
    }

    #[test]
    fn to_json_produces_valid_structure() {
        let err = SanctifierError::dry_run_no_changes("update");
        let json = to_json(&err);
        assert_eq!(json["error"]["code"], "E009");
        assert!(json["error"]["message"]
            .as_str()
            .unwrap()
            .contains("dry-run"));
        assert!(!json["error"]["hint"].as_str().unwrap().is_empty());
    }

    #[test]
    fn all_error_codes_have_stable_string_representation() {
        let codes = [
            (ErrorCode::E001, "E001"),
            (ErrorCode::E002, "E002"),
            (ErrorCode::E003, "E003"),
            (ErrorCode::E004, "E004"),
            (ErrorCode::E005, "E005"),
            (ErrorCode::E006, "E006"),
            (ErrorCode::E007, "E007"),
            (ErrorCode::E008, "E008"),
            (ErrorCode::E009, "E009"),
            (ErrorCode::E010, "E010"),
            (ErrorCode::E011, "E011"),
        ];
        for (code, expected) in codes {
            assert_eq!(code.as_str(), expected);
        }
    }

    #[test]
    fn invalid_network_hint_lists_valid_options() {
        let err = SanctifierError::invalid_network("devnet");
        assert!(
            err.message.contains("devnet"),
            "message should echo the bad value"
        );
        assert!(
            err.hint.contains("testnet"),
            "hint should list valid networks"
        );
        assert!(
            err.hint.contains("futurenet"),
            "hint should list valid networks"
        );
        assert!(
            err.hint.contains("mainnet"),
            "hint should list valid networks"
        );
        assert_eq!(err.code, ErrorCode::E010);
    }

    #[test]
    fn missing_credentials_hint_mentions_env_var_and_generate_command() {
        let err = SanctifierError::missing_credentials();
        assert!(err.hint.contains("SOROBAN_SECRET_KEY"));
        assert!(err.hint.contains("--secret-key"));
        assert!(err.hint.contains("generate"));
        assert_eq!(err.code, ErrorCode::E011);
    }

    #[test]
    fn e010_display_includes_code() {
        let err = SanctifierError::invalid_network("staging");
        let s = err.to_string();
        assert!(s.contains("[E010]"));
        assert!(s.contains("hint:"));
    }

    #[test]
    fn e011_display_includes_code() {
        let err = SanctifierError::missing_credentials();
        let s = err.to_string();
        assert!(s.contains("[E011]"));
        assert!(s.contains("hint:"));
    }
}
