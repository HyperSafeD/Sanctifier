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
        let content = include_str!("../../../data/vulnerability-db.json");
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
                let line_start = source[..mat.start()].rfind('\n').map(|p| p + 1).unwrap_or(0);
                let line_end = source[mat.end()..].find('\n').map(|p| mat.end() + p).unwrap_or(source.len());
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
