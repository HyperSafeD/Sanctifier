use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VulnEntry {
    pub id: String,
    pub name: String,
    pub description: String,
    pub severity: String,
    pub category: String,
    pub pattern: String,
    pub recommendation: String,
    #[serde(default)]
    pub references: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct VulnDatabase {
    pub version: String,
    pub last_updated: String,
    pub description: String,
    pub vulnerabilities: Vec<VulnEntry>,
}

#[derive(Debug, Clone, Serialize)]
pub struct VulnMatch {
    pub vuln_id: String,
    pub name: String,
    pub severity: String,
    pub category: String,
    pub description: String,
    pub recommendation: String,
    pub file: String,
    pub line: usize,
    pub snippet: String,
}

impl VulnDatabase {
    /// Load the vulnerability database from a JSON file.
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let content = fs::read_to_string(path)?;
        let db: VulnDatabase = serde_json::from_str(&content)?;
        Ok(db)
    }

    /// Load the embedded default vulnerability database.
    pub fn load_default() -> Self {
        let content = include_str!("../data/vulnerability-db.json");
        serde_json::from_str(content).expect("embedded vulnerability-db.json is valid")
    }

    /// Scan source code against all vulnerability patterns.
    pub fn scan(&self, source: &str, file_name: &str) -> Vec<VulnMatch> {
        let mut matches = Vec::new();

        for vuln in &self.vulnerabilities {
            let re = match Regex::new(&vuln.pattern) {
                Ok(r) => r,
                Err(_) => continue,
            };

            for mat in re.find_iter(source) {
                let line = source[..mat.start()].matches('\n').count() + 1;
                let line_start = source[..mat.start()]
                    .rfind('\n')
                    .map(|p| p + 1)
                    .unwrap_or(0);
                let line_end = source[mat.end()..]
                    .find('\n')
                    .map(|p| mat.end() + p)
                    .unwrap_or(source.len());
                let snippet = source[line_start..line_end].trim().to_string();

                matches.push(VulnMatch {
                    vuln_id: vuln.id.clone(),
                    name: vuln.name.clone(),
                    severity: vuln.severity.clone(),
                    category: vuln.category.clone(),
                    description: vuln.description.clone(),
                    recommendation: vuln.recommendation.clone(),
                    file: file_name.to_string(),
                    line,
                    snippet,
                });
            }
        }

        matches
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
