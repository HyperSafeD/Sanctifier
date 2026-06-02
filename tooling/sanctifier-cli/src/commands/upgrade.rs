use anyhow::{anyhow, Context};
use clap::Args;
use colored::Colorize;
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

const OWNER: &str = "HyperSafeD";
const REPO: &str = "Sanctifier";
const PACKAGE_NAME: &str = "sanctifier-cli";

#[derive(Args, Debug)]
pub struct UpgradeArgs {
    /// Only check for available version without downloading
    #[arg(long)]
    pub check: bool,
}

pub fn exec(args: UpgradeArgs) -> anyhow::Result<()> {
    let current = env!("CARGO_PKG_VERSION");
    println!(
        "{} {} v{}",
        "◆".bold(),
        "Sanctifier Upgrade".bold(),
        current
    );
    println!();

    let latest = fetch_latest_version()?;
    let latest_tag = latest
        .strip_prefix('v')
        .unwrap_or(&latest)
        .to_string();

    if !is_newer_version(current, &latest_tag) {
        println!(
            "{} Sanctifier is already up to date (v{}).",
            "✓".green().bold(),
            current
        );
        return Ok(());
    }

    println!(
        "  Current version : v{}",
        current.dimmed()
    );
    println!(
        "  Latest version  : v{}",
        latest_tag.green()
    );

    if args.check {
        println!();
        println!(
            "A new version is available. Run `sanctifier upgrade` to update."
        );
        return Ok(());
    }

    perform_upgrade(&latest_tag)
}

fn perform_upgrade(version: &str) -> anyhow::Result<()> {
    let target = target_triple()?;
    let archive_name = format!("{}-{}-{}.tar.gz", REPO, target, version);
    let download_url = format!(
        "https://github.com/{}/{}/releases/download/v{}/{}",
        OWNER, REPO, version, archive_name
    );
    let checksums_url = format!(
        "https://github.com/{}/{}/releases/download/v{}/CHECKSUMS.txt",
        OWNER, REPO, version
    );

    println!("  Downloading archive …");
    let archive_bytes = download_file(&download_url)
        .with_context(|| format!("Failed to download release archive from {}", download_url))?;

    println!("  Fetching checksums …");
    let checksums_text = download_file(&checksums_url)
        .with_context(|| format!("Failed to download CHECKSUMS.txt from {}", checksums_url))?;

    let expected_hash = extract_checksum(&checksums_text, &archive_name)
        .ok_or_else(|| anyhow!("{} not found in CHECKSUMS.txt", archive_name))?;

    println!("  Verifying SHA-256 checksum …");
    let actual_hash = hex::encode(Sha256::digest(&archive_bytes));
    if actual_hash != expected_hash {
        anyhow::bail!(
            "Checksum mismatch for {}:\n  expected: {}\n  actual:   {}",
            archive_name,
            expected_hash,
            actual_hash
        );
    }

    println!("  Checksum verified ✓");
    println!("  Extracting and installing binary …");

    let temp_dir = tempfile::tempdir().context("Failed to create temp directory")?;
    let archive_path = temp_dir.path().join(&archive_name);
    let mut f = std::fs::File::create(&archive_path)?;
    f.write_all(&archive_bytes)?;
    f.sync_all()?;
    drop(f);

    extract_tar_gz(&archive_path, temp_dir.path())?;

    let binary_name = if cfg!(windows) { "sanctifier.exe" } else { "sanctifier" };
    let extracted_binary = find_binary(temp_dir.path(), binary_name)
        .ok_or_else(|| anyhow!("Binary '{}' not found in extracted archive", binary_name))?;

    let current_exe = std::env::current_exe()
        .context("Could not determine current executable path")?;

    atomic_replace(&extracted_binary, &current_exe)?;

    println!();
    println!(
        "{} Sanctifier has been updated to v{}.",
        "✓".green().bold(),
        version
    );
    println!("  Binary: {}", current_exe.display());
    Ok(())
}

fn target_triple() -> anyhow::Result<String> {
    let arch = std::env::consts::ARCH;
    let os = std::env::consts::OS;
    let os_part = match os {
        "linux" => "unknown-linux-gnu",
        "macos" => "apple-darwin",
        "windows" => "pc-windows-msvc",
        _ => anyhow::bail!("Unsupported OS: {}", os),
    };
    Ok(format!("{}-{}", arch, os_part))
}

fn download_file(url: &str) -> anyhow::Result<Vec<u8>> {
    let client = reqwest::blocking::Client::builder()
        .user_agent("sanctifier-upgrade/1.0")
        .build()
        .context("Failed to create HTTP client")?;

    let response = client
        .get(url)
        .send()
        .with_context(|| format!("Failed to GET {}", url))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().unwrap_or_default();
        anyhow::bail!(
            "HTTP {} when fetching {}: {}",
            status,
            url,
            body.trim()
        );
    }

    response
        .bytes()
        .map(|b| b.to_vec())
        .context("Failed to read response body")
}

fn extract_checksum(text: &[u8], filename: &str) -> Option<String> {
    let content = std::str::from_utf8(text).ok()?;
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with('#') || line.is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.splitn(2, "  ").collect();
        if parts.len() == 2 && parts[1].trim() == filename {
            return Some(parts[0].trim().to_lowercase());
        }
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 && parts[1] == filename {
            return Some(parts[0].to_lowercase());
        }
    }
    None
}

fn extract_tar_gz(archive_path: &Path, dest: &Path) -> anyhow::Result<()> {
    let file = std::fs::File::open(archive_path)?;
    let decoder = flate2::read::GzDecoder::new(file);
    let mut archive = tar::Archive::new(decoder);
    archive.unpack(dest).context("Failed to extract archive")
}

fn find_binary(dir: &Path, name: &str) -> Option<PathBuf> {
    for entry in walkdir::WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_file() {
            if entry.file_name().to_string_lossy() == name {
                return Some(entry.path().to_path_buf());
            }
        }
    }
    None
}

fn atomic_replace(source: &Path, target: &Path) -> anyhow::Result<()> {
    let backup = target.with_extension("old.bak");

    if backup.exists() {
        std::fs::remove_file(&backup)?;
    }

    std::fs::rename(target, &backup)?;

    match std::fs::rename(source, target) {
        Ok(()) => {
            let _ = std::fs::remove_file(&backup);
            Ok(())
        }
        Err(e) => {
            let _ = std::fs::rename(&backup, target);
            Err(anyhow!("Failed to replace binary: {}", e))
        }
    }
}

fn fetch_latest_version() -> anyhow::Result<String> {
    let api_url = format!(
        "https://api.github.com/repos/{}/{}/releases/latest",
        OWNER, REPO
    );

    let client = reqwest::blocking::Client::builder()
        .user_agent("sanctifier-upgrade/1.0")
        .build()
        .context("Failed to create HTTP client")?;

    let response = client
        .get(&api_url)
        .send()
        .with_context(|| format!("Failed to fetch latest release from {}", api_url))?;

    if !response.status().is_success() {
        anyhow::bail!(
            "GitHub API returned HTTP {} when fetching latest release",
            response.status()
        );
    }

    #[derive(serde::Deserialize)]
    struct Release {
        tag_name: String,
    }

    let release: Release = response
        .json()
        .context("Failed to parse GitHub release JSON")?;

    Ok(release.tag_name)
}

fn is_newer_version(current: &str, latest: &str) -> bool {
    match (parse_triplet(current), parse_triplet(latest)) {
        (Some(cur), Some(new)) => new > cur,
        _ => current.trim() != latest.trim(),
    }
}

fn parse_triplet(version: &str) -> Option<(u64, u64, u64)> {
    let version = version.strip_prefix('v').unwrap_or(version);
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

#[cfg(test)]
mod tests {
    use super::{extract_checksum, is_newer_version, parse_triplet};

    #[test]
    fn parse_triplet_parses_semver_values() {
        assert_eq!(parse_triplet("1.2.3"), Some((1, 2, 3)));
        assert_eq!(parse_triplet("v1.2.3"), Some((1, 2, 3)));
        assert_eq!(parse_triplet("1.2.3-beta.1"), Some((1, 2, 3)));
        assert_eq!(parse_triplet("1.2"), None);
    }

    #[test]
    fn extract_checksum_parses_standard_format() {
        let text = b"137880142281345efe8668eada85cca0a1979ed2fbb2cf099629ad62eeaba95e  sanctifier-x86_64-unknown-linux-gnu-0.1.0.tar.gz\n";
        let result = extract_checksum(text, "sanctifier-x86_64-unknown-linux-gnu-0.1.0.tar.gz");
        assert_eq!(
            result,
            Some("137880142281345efe8668eada85cca0a1979ed2fbb2cf099629ad62eeaba95e".to_string())
        );
    }

    #[test]
    fn extract_checksum_skips_comments() {
        let text = b"# Sanctifier Release Artifact Checksums\n137880142281345efe8668eada85cca0a1979ed2fbb2cf099629ad62eeaba95e  file.tar.gz\n";
        let result = extract_checksum(text, "file.tar.gz");
        assert!(result.is_some());
    }

    #[test]
    fn extract_checksum_returns_none_for_missing() {
        let text = b"checksum  other-file.tar.gz\n";
        let result = extract_checksum(text, "missing-file.tar.gz");
        assert!(result.is_none());
    }

    #[test]
    fn version_compare_prefers_higher_triplet() {
        assert!(is_newer_version("0.1.0", "0.2.0"));
        assert!(!is_newer_version("0.3.0", "0.2.9"));
        assert!(!is_newer_version("0.1.0", "0.1.0"));
    }
}
