use anyhow::{anyhow, Context};
use std::process::Command;

const PACKAGE_NAME: &str = "sanctifier-cli";

pub fn exec() -> anyhow::Result<()> {
    let current = env!("CARGO_PKG_VERSION");
    println!("Checking for Sanctifier updates (current: v{current})...");

    let latest = fetch_latest_version()?;
    if !is_newer_version(current, &latest) {
        println!("Sanctifier is already up to date (v{current}).");
        return Ok(());
    }

    println!("Updating Sanctifier from v{current} to v{latest}...");
    install_version(&latest)?;
    println!("Update complete. Sanctifier is now at version v{latest}.");
    Ok(())
}

fn fetch_latest_version() -> anyhow::Result<String> {
    let output = Command::new("cargo")
        .args(["search", PACKAGE_NAME, "--limit", "1"])
        .output()
        .context("failed to run `cargo search`")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!("`cargo search` failed: {}", stderr.trim()));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_latest_version(&stdout)
}

fn parse_latest_version(output: &str) -> anyhow::Result<String> {
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

    Err(anyhow!(
        "could not parse latest sanctifier-cli version from cargo search output"
    ))
}

fn install_version(version: &str) -> anyhow::Result<()> {
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
        Err(anyhow!(
            "`cargo install` failed while installing sanctifier-cli v{}",
            version
        ))
    }
}

fn parse_triplet(version: &str) -> Option<(u64, u64, u64)> {
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

fn is_newer_version(current: &str, latest: &str) -> bool {
    match (parse_triplet(current), parse_triplet(latest)) {
        (Some(cur), Some(new)) => new > cur,
        _ => current.trim() != latest.trim(),
    }
}

#[cfg(test)]
mod tests {
    use super::{is_newer_version, parse_latest_version, parse_triplet};

    #[test]
    fn parse_triplet_parses_semver_values() {
        assert_eq!(parse_triplet("1.2.3"), Some((1, 2, 3)));
        assert_eq!(parse_triplet("1.2.3-beta.1"), Some((1, 2, 3)));
        assert_eq!(parse_triplet("1.2"), None);
    }

    #[test]
    fn parse_latest_version_extracts_first_match() {
        let output = "sanctifier-cli = \"0.3.4\"    # Sanctifier CLI";
        let version = parse_latest_version(output).unwrap();
        assert_eq!(version, "0.3.4");
    }

    #[test]
    fn parse_latest_version_errors_on_missing_match() {
        let output = "something-else = \"1.0.0\"";
        assert!(parse_latest_version(output).is_err());
    }

    #[test]
    fn version_compare_prefers_higher_triplet() {
        assert!(is_newer_version("0.1.0", "0.2.0"));
        assert!(!is_newer_version("0.3.0", "0.2.9"));
        assert!(!is_newer_version("0.1.0", "0.1.0"));
    }
}
