use serde::{Deserialize, Serialize};
use std::path::Path;

/// SDK version information and deprecation warnings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SdkVersionInfo {
    /// Detected SDK version (e.g., "21.7.6")
    pub version: Option<String>,
    /// Whether the version is deprecated
    pub is_deprecated: bool,
    /// Deprecation warnings
    pub warnings: Vec<String>,
    /// Recommended version
    pub recommended_version: Option<String>,
}

impl SdkVersionInfo {
    /// Return an SDK version result for cases where no version could be detected.
    pub fn unknown() -> Self {
        Self {
            version: None,
            is_deprecated: false,
            warnings: vec![],
            recommended_version: None,
        }
    }
}

/// Detect Soroban SDK version from Cargo.toml
pub fn detect_sdk_version(cargo_toml_path: &Path) -> SdkVersionInfo {
    let content = match std::fs::read_to_string(cargo_toml_path) {
        Ok(c) => c,
        Err(_) => return SdkVersionInfo::unknown(),
    };

    let version = extract_soroban_sdk_version(&content);

    match version {
        Some(v) => analyze_version(&v),
        None => SdkVersionInfo::unknown(),
    }
}

fn extract_soroban_sdk_version(cargo_toml: &str) -> Option<String> {
    // Parse TOML to find soroban-sdk version
    for line in cargo_toml.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("soroban-sdk") || trimmed.starts_with("soroban_sdk") {
            // Extract version from patterns like:
            // soroban-sdk = "21.7.6"
            // soroban-sdk = { version = "21.7.6" }
            if let Some(version_start) = trimmed.find('"') {
                if let Some(version_end) = trimmed[version_start + 1..].find('"') {
                    let version = &trimmed[version_start + 1..version_start + 1 + version_end];
                    return Some(version.to_string());
                }
            }
        }
    }
    None
}

fn analyze_version(version: &str) -> SdkVersionInfo {
    let mut info = SdkVersionInfo {
        version: Some(version.to_string()),
        is_deprecated: false,
        warnings: vec![],
        recommended_version: Some("21.7.6".to_string()),
    };

    // Parse version
    let parts: Vec<&str> = version.split('.').collect();
    if parts.len() < 2 {
        return info;
    }

    let major: u32 = parts[0].parse().unwrap_or(0);
    let minor: u32 = parts[1].parse().unwrap_or(0);

    // Check for deprecated versions
    if major < 20 {
        info.is_deprecated = true;
        info.warnings.push(format!(
            "Soroban SDK {} is severely outdated. Upgrade to {} for security fixes and new features.",
            version, info.recommended_version.as_ref().unwrap()
        ));
        info.warnings.push(
            "Pre-v20 SDKs lack critical security features and are not compatible with current Soroban networks.".to_string()
        );
    } else if major == 20 {
        info.is_deprecated = true;
        info.warnings.push(format!(
            "Soroban SDK {} is deprecated. Consider upgrading to {} for improved performance and security.",
            version, info.recommended_version.as_ref().unwrap()
        ));
        info.warnings.push(
            "SDK v20 has known issues with storage TTL management and event emission.".to_string(),
        );
    } else if major == 21 && minor < 5 {
        info.warnings.push(format!(
            "Soroban SDK {} has known issues. Upgrade to {} recommended.",
            version,
            info.recommended_version.as_ref().unwrap()
        ));
        info.warnings.push(
            "Early v21 releases had bugs in authorization and cross-contract calls.".to_string(),
        );
    }

    // Check for insecure patterns in specific versions
    if version == "21.0.0" || version == "21.1.0" {
        info.warnings.push(
            "This SDK version has a critical bug in require_auth() - upgrade immediately!"
                .to_string(),
        );
    }

    info
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_version_simple() {
        let toml = r#"
[dependencies]
soroban-sdk = "21.7.6"
        "#;
        let version = extract_soroban_sdk_version(toml);
        assert_eq!(version, Some("21.7.6".to_string()));
    }

    #[test]
    fn test_extract_version_with_features() {
        let toml = r#"
[dependencies]
soroban-sdk = { version = "21.7.6", features = ["testutils"] }
        "#;
        let version = extract_soroban_sdk_version(toml);
        assert_eq!(version, Some("21.7.6".to_string()));
    }

    #[test]
    fn test_analyze_deprecated_version() {
        let info = analyze_version("19.5.0");
        assert!(info.is_deprecated);
        assert!(!info.warnings.is_empty());
    }

    #[test]
    fn test_analyze_current_version() {
        let info = analyze_version("21.7.6");
        assert!(!info.is_deprecated);
    }

    #[test]
    fn test_analyze_early_v21() {
        let info = analyze_version("21.2.0");
        assert!(!info.warnings.is_empty());
    }
}
