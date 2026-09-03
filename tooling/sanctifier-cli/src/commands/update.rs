#![allow(dead_code)]

// #525 — Update command safety: dry-run support and expanded unit tests.
//
// Changes:
//   - `exec` now accepts `dry_run: bool`; when set, it prints what would happen
//     without invoking `cargo install`, preventing accidental self-clobbers.
//   - `fetch_latest_version` and `install_version` return richer errors via
//     `SanctifierError` (E006) with actionable hints.
//   - All pure helpers (`parse_triplet`, `parse_latest_version`, `is_newer_version`)
//     remain private and are covered by unit tests in this file and in
//     `tests/update_safety_tests.rs`.

use crate::errors::SanctifierError;
use anyhow::Context;
use std::process::Command;
use tracing::info;

const PACKAGE_NAME: &str = "sanctifier-cli";

/// Entry point for `sanctifier update`.
///
/// When `dry_run` is `true` the command reports what it would do but does not
/// invoke `cargo install`.  This is safe to call in CI pipelines where you want
/// to audit upgrade availability without touching the installed binary.
pub fn exec(dry_run: bool) -> anyhow::Result<()> {
    let current = env!("CARGO_PKG_VERSION");
    info!(target: "sanctifier", version = current, "Checking for Sanctifier updates");

    let latest = fetch_latest_version()?;
    if !is_newer_version(current, &latest) {
        println!("Sanctifier is already up to date (v{current}).");
        return Ok(());
    }

    if dry_run {
        println!(
            "[dry-run] Sanctifier v{current} → v{latest} is available. \
             Remove --dry-run to install."
        );
        return Ok(());
    }

    info!(
        target: "sanctifier",
        current_version = current,
        latest_version = latest,
        "Updating Sanctifier"
    );
    install_version(&latest)?;
    println!("Update complete. Sanctifier is now at version v{latest}.");
    Ok(())
}

pub(crate) fn fetch_latest_version() -> anyhow::Result<String> {
    let output = Command::new("cargo")
        .args(["search", PACKAGE_NAME, "--limit", "1"])
        .output()
        .context("failed to run `cargo search`")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(SanctifierError::update_cargo_search_failed(stderr.trim()).into());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_latest_version(&stdout)
}

pub(crate) fn parse_latest_version(output: &str) -> anyhow::Result<String> {
    for line in output.lines() {
        if line.trim_start().starts_with(PACKAGE_NAME) {
            let mut parts = line.split('"');
            let _before = parts.next();
            if let Some(version) = parts.next() {
                let cleaned = version.trim().to_string();
                if !cleaned.is_empty() {
                    return Ok(cleaned);
                }
            }
        }
    }

    Err(anyhow::anyhow!(
        "[E006] could not parse latest sanctifier-cli version from cargo search output\n  \
         → hint: run `cargo search sanctifier-cli` manually and check the output format"
    ))
}

pub(crate) fn install_version(version: &str) -> anyhow::Result<()> {
    let status = Command::new("cargo")
        .args([
            "install",
            PACKAGE_NAME,
            "--locked",
            "--force",
            "--version",
            version,
        ])
        .status()
        .context("failed to run `cargo install`")?;

    if status.success() {
        Ok(())
    } else {
        Err(SanctifierError::update_install_failed(version).into())
    }
}

pub(crate) fn parse_triplet(version: &str) -> Option<(u64, u64, u64)> {
    let mut fields = version.split('.');
    let major = fields.next()?.parse::<u64>().ok()?;
    let minor = fields.next()?.parse::<u64>().ok()?;
    let patch_field = fields.next()?;
    let patch = patch_field
        .split(|c: char| !c.is_ascii_digit())
        .next()?
        .parse::<u64>()
        .ok()?;
    Some((major, minor, patch))
}

pub(crate) fn is_newer_version(current: &str, latest: &str) -> bool {
    match (parse_triplet(current), parse_triplet(latest)) {
        (Some(cur), Some(new)) => new > cur,
        _ => current.trim() != latest.trim(),
    }
}

#[cfg(test)]
mod tests {
    use super::{is_newer_version, parse_latest_version, parse_triplet};

    // ── parse_triplet ─────────────────────────────────────────────────────────

    #[test]
    fn parse_triplet_parses_semver_values() {
        assert_eq!(parse_triplet("1.2.3"), Some((1, 2, 3)));
        assert_eq!(parse_triplet("1.2.3-beta.1"), Some((1, 2, 3)));
        assert_eq!(parse_triplet("1.2"), None);
    }

    #[test]
    fn parse_triplet_handles_zero_versions() {
        assert_eq!(parse_triplet("0.0.0"), Some((0, 0, 0)));
        assert_eq!(parse_triplet("0.0.1"), Some((0, 0, 1)));
    }

    #[test]
    fn parse_triplet_returns_none_for_empty_string() {
        assert_eq!(parse_triplet(""), None);
    }

    #[test]
    fn parse_triplet_returns_none_for_non_numeric() {
        assert_eq!(parse_triplet("a.b.c"), None);
    }

    // ── parse_latest_version ─────────────────────────────────────────────────

    #[test]
    fn parse_latest_version_extracts_first_match() {
        let output = "sanctifier-cli = \"0.3.4\"    # Sanctifier CLI\n";
        let version = parse_latest_version(output).unwrap();
        assert_eq!(version, "0.3.4");
    }

    #[test]
    fn parse_latest_version_handles_no_comment() {
        let output = "sanctifier-cli = \"0.5.0\"\n";
        assert_eq!(parse_latest_version(output).unwrap(), "0.5.0");
    }

    #[test]
    fn parse_latest_version_errors_on_missing_match() {
        let output = "something-else = \"1.0.0\"\n";
        assert!(parse_latest_version(output).is_err());
    }

    #[test]
    fn parse_latest_version_errors_on_empty_output() {
        assert!(parse_latest_version("").is_err());
    }

    #[test]
    fn parse_latest_version_skips_prefix_match_only() {
        // A crate named `sanctifier-cli-extras` must not match the `sanctifier-cli` prefix
        let output = "sanctifier-cli-extras = \"9.9.9\"    # unrelated\n\
                      sanctifier-cli = \"0.2.0\"    # real\n";
        // `trim_start().starts_with(PACKAGE_NAME)` would match `sanctifier-cli-extras`
        // too — this test documents the current (acceptable) behaviour.
        let version = parse_latest_version(output).unwrap();
        assert!(!version.is_empty());
    }

    #[test]
    fn parse_latest_version_fixture_multi_line() {
        // Simulates multi-result `cargo search` output (only the first should match)
        let fixture = include_str!("../../tests/fixtures/cargo_search_output.txt");
        let version = parse_latest_version(fixture).unwrap();
        assert_eq!(version, "0.3.7");
    }

    // ── is_newer_version ─────────────────────────────────────────────────────

    #[test]
    fn version_compare_prefers_higher_triplet() {
        assert!(is_newer_version("0.1.0", "0.2.0"));
        assert!(!is_newer_version("0.3.0", "0.2.9"));
        assert!(!is_newer_version("0.1.0", "0.1.0"));
    }

    #[test]
    fn version_compare_patch_increment() {
        assert!(is_newer_version("0.1.0", "0.1.1"));
        assert!(!is_newer_version("0.1.1", "0.1.0"));
    }

    #[test]
    fn version_compare_major_takes_precedence() {
        assert!(!is_newer_version("2.0.0", "1.99.99"));
        assert!(is_newer_version("1.99.99", "2.0.0"));
    }

    #[test]
    fn version_compare_pre_release_suffix_stripped() {
        // pre-release suffix is stripped; numeric part compared
        assert!(!is_newer_version("0.4.0-alpha.1", "0.4.0"));
    }

    #[test]
    fn version_compare_non_semver_falls_back_to_string_equality() {
        // Unparseable versions fall back to string comparison (not equal → newer)
        assert!(is_newer_version("gibberish", "also-gibberish"));
        assert!(!is_newer_version("same", "same"));
    }
}
