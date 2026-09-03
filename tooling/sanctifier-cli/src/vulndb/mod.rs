#![allow(dead_code)]
//! Vulnerability database — loading, validation, and pattern matching.
//!
//! # Module layout
//!
//! | Submodule | Responsibility |
//! |-----------|----------------|
//! | (this file) | Database types, JSON loading, semantic validation |
//! | [`matcher`] | Regex scan engine and [`VulnMatch`] result type |
//!
//! ## Threat model
//!
//! The vulnerability database is an untrusted external input (especially when
//! loaded from a user-supplied `--vuln-db` path).  [`VulnDatabase::validate`]
//! runs before any scanning to:
//!
//! 1. Reject databases whose entries contain invalid regular expressions,
//!    preventing a malformed pattern from panicking inside `regex::Regex::new`.
//! 2. Enforce unique IDs and non-overlapping signatures so that a crafted DB
//!    cannot produce duplicate or misleading findings.
//! 3. Reject unknown severity strings to keep downstream consumers (JSON
//!    output, CI exit-code logic) from seeing unexpected values.
//!
//! The embedded default database (`data/vulnerability-db.json`) is validated
//! at compile-time via `expect` — a bug in the embedded DB causes a build
//! failure, not a runtime error.

pub mod matcher;

pub use matcher::VulnMatch;

use anyhow::Context;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// A single entry in the vulnerability database.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VulnEntry {
    /// Unique identifier (e.g. `"VULN-001"`).
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// Human-readable description.
    pub description: String,
    /// Severity level: one of `critical`, `high`, `medium`, `low`, `info`.
    pub severity: String,
    /// Broad vulnerability category.
    pub category: String,
    /// Regex pattern matched against source code.
    pub pattern: String,
    /// Actionable recommendation.
    pub recommendation: String,
    /// Optional external references (CVEs, advisories, …).
    #[serde(default)]
    pub references: Vec<String>,
}

/// A parsed and validated vulnerability database.
#[derive(Debug, Clone, Deserialize)]
pub struct VulnDatabase {
    /// Schema version of this database file.
    pub version: String,
    /// ISO-8601 date of the last update.
    pub last_updated: String,
    /// Human-readable description of the database.
    pub description: String,
    /// All vulnerability entries.
    pub vulnerabilities: Vec<VulnEntry>,
}

impl VulnDatabase {
    /// Load a vulnerability database from a JSON file on disk.
    ///
    /// The file is parsed and then semantically validated via [`Self::validate`].
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("failed to read vulnerability database {}", path.display()))?;
        let db: VulnDatabase = serde_json::from_str(&content).with_context(|| {
            format!(
                "failed to parse vulnerability database JSON {}",
                path.display()
            )
        })?;
        db.validate().with_context(|| {
            format!(
                "vulnerability database failed semantic validation {}",
                path.display()
            )
        })?;
        Ok(db)
    }

    /// Load the embedded default vulnerability database (compiled into the binary).
    ///
    /// Panics at startup if the embedded JSON is invalid — this is intentional
    /// because a broken embedded database is a build defect, not a runtime one.
    pub fn load_default() -> Self {
        let content = include_str!("../../data/vulnerability-db.json");
        let db: VulnDatabase =
            serde_json::from_str(content).expect("embedded vulnerability-db.json is valid JSON");
        db.validate()
            .expect("embedded vulnerability-db.json passes semantic validation");
        db
    }

    /// Validate uniqueness and semantic constraints that JSON Schema cannot express.
    ///
    /// Returns an error listing **all** validation failures so that users can
    /// fix their custom database in one pass rather than chasing errors one by one.
    pub fn validate(&self) -> anyhow::Result<()> {
        if self.version.trim().is_empty() {
            anyhow::bail!("vulnerability database version must not be empty");
        }
        if self.last_updated.trim().is_empty() {
            anyhow::bail!("vulnerability database last_updated must not be empty");
        }
        if self.description.trim().is_empty() {
            anyhow::bail!("vulnerability database description must not be empty");
        }
        if self.vulnerabilities.is_empty() {
            anyhow::bail!("vulnerability database must contain at least one vulnerability");
        }

        let allowed_severities = ["critical", "high", "medium", "low", "info"];
        let id_re = Regex::new(r"^[A-Z0-9][A-Z0-9._-]*$").expect("id regex is valid");

        let mut ids: HashMap<&str, usize> = HashMap::new();
        let mut names: HashMap<String, usize> = HashMap::new();
        let mut signatures: HashMap<String, usize> = HashMap::new();
        let mut errors = Vec::new();

        for (index, vuln) in self.vulnerabilities.iter().enumerate() {
            let id_trimmed = vuln.id.trim();
            if id_trimmed.is_empty() {
                errors.push(format!("vulnerabilities[{index}].id must not be empty"));
            } else if !id_re.is_match(id_trimmed) {
                errors.push(format!(
                    "vulnerabilities[{index}].id must match {} (got {:?})",
                    id_re.as_str(),
                    vuln.id
                ));
            }
            if vuln.name.trim().is_empty() {
                errors.push(format!("vulnerabilities[{index}].name must not be empty"));
            }
            if vuln.description.trim().is_empty() {
                errors.push(format!(
                    "vulnerabilities[{index}].description must not be empty"
                ));
            }
            if vuln.recommendation.trim().is_empty() {
                errors.push(format!(
                    "vulnerabilities[{index}].recommendation must not be empty"
                ));
            }

            let severity_norm = vuln.severity.trim().to_ascii_lowercase();
            if severity_norm.is_empty() {
                errors.push(format!(
                    "vulnerabilities[{index}].severity must not be empty"
                ));
            } else if !allowed_severities.contains(&severity_norm.as_str()) {
                errors.push(format!(
                    "vulnerabilities[{index}].severity must be one of {}, got {:?}",
                    allowed_severities.join(", "),
                    vuln.severity
                ));
            }

            if vuln.category.trim().is_empty() {
                errors.push(format!(
                    "vulnerabilities[{index}].category must not be empty"
                ));
            }
            if vuln.pattern.trim().is_empty() {
                errors.push(format!(
                    "vulnerabilities[{index}].pattern must not be empty"
                ));
            } else if let Err(err) = Regex::new(&vuln.pattern) {
                let id_display = if id_trimmed.is_empty() {
                    "<missing id>"
                } else {
                    id_trimmed
                };
                errors.push(format!(
                    "{id_display} has invalid regex pattern at vulnerabilities[{index}].pattern: {err}",
                ));
            }

            if let Some(first) = ids.insert(vuln.id.as_str(), index) {
                errors.push(format!(
                    "duplicate vulnerability id {:?} at vulnerabilities[{first}] and vulnerabilities[{index}]",
                    vuln.id
                ));
            }

            let name_key = vuln.name.trim().to_ascii_lowercase();
            if !name_key.is_empty() {
                if let Some(first) = names.insert(name_key, index) {
                    errors.push(format!(
                        "duplicate vulnerability name {:?} at vulnerabilities[{first}] and vulnerabilities[{index}]",
                        vuln.name
                    ));
                }
            }

            let signature = format!(
                "{}\x1f{}\x1f{}",
                vuln.category.trim().to_ascii_lowercase(),
                severity_norm,
                vuln.pattern.trim()
            );
            if !vuln.pattern.trim().is_empty() {
                if let Some(first) = signatures.insert(signature, index) {
                    errors.push(format!(
                        "overlapping vulnerability signature between {} at vulnerabilities[{first}] and {} at vulnerabilities[{index}]",
                        self.vulnerabilities[first].id, vuln.id
                    ));
                }
            }
        }

        if !errors.is_empty() {
            anyhow::bail!("invalid vulnerability database:\n{}", errors.join("\n"));
        }

        Ok(())
    }

    /// Scan `source` against all vulnerability patterns.
    ///
    /// Delegates to [`matcher::scan_source`] keeping I/O and matching separate.
    pub fn scan(&self, source: &str, file_name: &str) -> Vec<VulnMatch> {
        matcher::scan_source(&self.vulnerabilities, source, file_name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_load_default_database_succeeds() {
        let db = VulnDatabase::load_default();
        assert!(
            !db.vulnerabilities.is_empty(),
            "Default database should contain vulnerabilities"
        );
        assert!(!db.version.is_empty(), "Database should have a version");
    }

    #[test]
    fn test_load_default_database_has_required_entries() {
        let db = VulnDatabase::load_default();
        let ids: Vec<&str> = db.vulnerabilities.iter().map(|v| v.id.as_str()).collect();
        assert!(
            ids.contains(&"VULN-001"),
            "Database should contain VULN-001"
        );
        assert!(
            ids.contains(&"VULN-002"),
            "Database should contain VULN-002"
        );
        assert!(
            ids.contains(&"VULN-003"),
            "Database should contain VULN-003"
        );
    }

    #[test]
    fn test_scan_detects_panic_pattern() {
        let db = VulnDatabase::load_default();
        let source = r#"
fn example() {
    panic!("this is a panic");
}
"#;
        let matches = db.scan(source, "test.rs");
        assert!(!matches.is_empty(), "Should detect panic! usage");
        assert!(
            matches.iter().any(|m| m.vuln_id == "VULN-002"),
            "Should match VULN-002 for panic"
        );
    }

    #[test]
    fn test_scan_detects_unwrap_pattern() {
        let db = VulnDatabase::load_default();
        let source = r#"
fn example(x: Option<i32>) -> i32 {
    x.unwrap()
}
"#;
        let matches = db.scan(source, "test.rs");
        assert!(!matches.is_empty(), "Should detect unwrap() usage");
        assert!(
            matches.iter().any(|m| m.vuln_id == "VULN-002"),
            "Should match VULN-002 for unwrap"
        );
    }

    #[test]
    fn test_scan_detects_unsafe_block() {
        let db = VulnDatabase::load_default();
        let source = r#"
fn example() {
    unsafe {
        let x = *ptr;
    }
}
"#;
        let matches = db.scan(source, "test.rs");
        assert!(
            matches.iter().any(|m| m.vuln_id == "VULN-004"),
            "Should match VULN-004 for unsafe block"
        );
    }

    #[test]
    fn test_scan_returns_empty_for_clean_code() {
        let db = VulnDatabase::load_default();
        let source = r#"
pub fn safe_function() {
    let x = "hello";
}
"#;
        let matches = db.scan(source, "clean.rs");
        assert!(
            matches.is_empty(),
            "Should return no matches for safe code, got: {:?}",
            matches
        );
    }

    #[test]
    fn test_scan_reports_correct_line_numbers() {
        let db = VulnDatabase::load_default();
        let source = r#"fn first() {}

fn second() {
    panic!("error here");
}
"#;
        let matches = db.scan(source, "test.rs");
        let panic_match = matches.iter().find(|m| m.vuln_id == "VULN-002");
        assert!(panic_match.is_some(), "Should find panic match");
        assert_eq!(panic_match.unwrap().line, 4, "Panic should be on line 4");
    }

    #[test]
    fn test_load_custom_database_from_file() {
        let custom_db_content = r#"{
            "version": "0.1.0",
            "last_updated": "2026-01-01",
            "description": "Custom test database",
            "vulnerabilities": [
                {
                    "id": "CUSTOM-001",
                    "name": "Test Vulnerability",
                    "description": "A test vulnerability",
                    "severity": "low",
                    "category": "test",
                    "pattern": "test_pattern",
                    "recommendation": "Fix it"
                }
            ]
        }"#;

        let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
        temp_file
            .write_all(custom_db_content.as_bytes())
            .expect("Failed to write temp file");
        temp_file.flush().expect("Failed to flush");

        let db = VulnDatabase::load(temp_file.path()).expect("Failed to load custom database");
        assert_eq!(db.version, "0.1.0");
        assert_eq!(db.vulnerabilities.len(), 1);
        assert_eq!(db.vulnerabilities[0].id, "CUSTOM-001");
    }

    #[test]
    fn test_validate_rejects_duplicate_ids() {
        let db = VulnDatabase {
            version: "0.1.0".to_string(),
            last_updated: "2026-04-23".to_string(),
            description: "duplicate ids".to_string(),
            vulnerabilities: vec![
                VulnEntry {
                    id: "DUP-001".to_string(),
                    name: "First".to_string(),
                    description: "first".to_string(),
                    severity: "low".to_string(),
                    category: "test".to_string(),
                    pattern: "first".to_string(),
                    recommendation: "fix first".to_string(),
                    references: vec![],
                },
                VulnEntry {
                    id: "DUP-001".to_string(),
                    name: "Second".to_string(),
                    description: "second".to_string(),
                    severity: "low".to_string(),
                    category: "test".to_string(),
                    pattern: "second".to_string(),
                    recommendation: "fix second".to_string(),
                    references: vec![],
                },
            ],
        };

        let err = db.validate().expect_err("duplicate IDs should fail");
        assert!(err.to_string().contains("duplicate vulnerability id"));
    }

    #[test]
    fn test_validate_rejects_overlapping_signatures() {
        let db = VulnDatabase {
            version: "0.1.0".to_string(),
            last_updated: "2026-04-23".to_string(),
            description: "overlapping signatures".to_string(),
            vulnerabilities: vec![
                VulnEntry {
                    id: "SIG-001".to_string(),
                    name: "First".to_string(),
                    description: "first".to_string(),
                    severity: "high".to_string(),
                    category: "auth".to_string(),
                    pattern: "require_auth".to_string(),
                    recommendation: "fix first".to_string(),
                    references: vec![],
                },
                VulnEntry {
                    id: "SIG-002".to_string(),
                    name: "Second".to_string(),
                    description: "second".to_string(),
                    severity: "HIGH".to_string(),
                    category: "AUTH".to_string(),
                    pattern: "require_auth".to_string(),
                    recommendation: "fix second".to_string(),
                    references: vec![],
                },
            ],
        };

        let err = db
            .validate()
            .expect_err("overlapping signatures should fail");
        assert!(err
            .to_string()
            .contains("overlapping vulnerability signature"));
    }

    #[test]
    fn test_load_rejects_invalid_regex_with_context() {
        let custom_db_content = r#"{
            "version": "0.1.0",
            "last_updated": "2026-04-23",
            "description": "Invalid regex database",
            "vulnerabilities": [
                {
                    "id": "BAD-REGEX",
                    "name": "Bad Regex",
                    "description": "A bad regex",
                    "severity": "low",
                    "category": "test",
                    "pattern": "(",
                    "recommendation": "Fix it"
                }
            ]
        }"#;

        let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
        temp_file
            .write_all(custom_db_content.as_bytes())
            .expect("Failed to write temp file");
        temp_file.flush().expect("Failed to flush");

        let err = VulnDatabase::load(temp_file.path()).expect_err("invalid regex should fail");
        let err_chain = format!("{err:#}");
        assert!(
            err_chain.contains("BAD-REGEX has invalid regex pattern"),
            "expected error chain to contain 'BAD-REGEX has invalid regex pattern', got: {err_chain}"
        );
    }

    #[test]
    fn test_load_rejects_invalid_severity_with_context() {
        let custom_db_content = r#"{
            "version": "0.1.0",
            "last_updated": "2026-04-23",
            "description": "Invalid severity database",
            "vulnerabilities": [
                {
                    "id": "BAD-SEVERITY",
                    "name": "Bad Severity",
                    "description": "A bad severity",
                    "severity": "urgent",
                    "category": "test",
                    "pattern": "urgent",
                    "recommendation": "Fix it"
                }
            ]
        }"#;

        let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
        temp_file
            .write_all(custom_db_content.as_bytes())
            .expect("Failed to write temp file");
        temp_file.flush().expect("Failed to flush");

        let err = VulnDatabase::load(temp_file.path()).expect_err("invalid severity should fail");
        let err_chain = format!("{err:#}");
        assert!(
            err_chain.contains("vulnerabilities[0].severity"),
            "expected error chain to contain 'vulnerabilities[0].severity', got: {err_chain}"
        );
        assert!(
            err_chain.contains("urgent"),
            "expected error chain to contain 'urgent', got: {err_chain}"
        );
        assert!(err_chain.contains("vulnerabilities[0].severity"));
        assert!(err_chain.contains("urgent"));
    }

    #[test]
    fn test_vuln_entry_has_all_required_fields() {
        let db = VulnDatabase::load_default();
        for vuln in &db.vulnerabilities {
            assert!(!vuln.id.is_empty(), "Vulnerability should have an id");
            assert!(!vuln.name.is_empty(), "Vulnerability should have a name");
            assert!(
                !vuln.severity.is_empty(),
                "Vulnerability should have a severity"
            );
            assert!(
                !vuln.category.is_empty(),
                "Vulnerability should have a category"
            );
            assert!(
                !vuln.pattern.is_empty(),
                "Vulnerability should have a pattern"
            );
            assert!(
                !vuln.recommendation.is_empty(),
                "Vulnerability should have a recommendation"
            );
        }
    }

    #[test]
    fn test_vuln_match_contains_file_info() {
        let db = VulnDatabase::load_default();
        let source = r#"panic!("test");"#;
        let matches = db.scan(source, "my_file.rs");
        assert!(!matches.is_empty());
        assert_eq!(matches[0].file, "my_file.rs");
        assert!(!matches[0].snippet.is_empty());
    }
}
