//! Pattern-matching engine for the vulnerability database.
//!
//! This module owns the [`VulnMatch`] result type and the [`scan_source`]
//! function that runs every [`super::VulnEntry`] regex pattern against a
//! source file.  Keeping the matching logic separate from the database I/O in
//! [`super`] makes the boundary between "loading data" and "using data" clear
//! and allows the scanner to be unit-tested without touching the file system.

use regex::Regex;
use serde::{Deserialize, Serialize};

use super::VulnEntry;

/// A single pattern match from the vulnerability database scan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VulnMatch {
    /// Unique identifier of the matched vulnerability (e.g. `"VULN-001"`).
    pub vuln_id: String,
    /// Human-readable name of the vulnerability.
    pub name: String,
    /// Severity level string (one of `critical`, `high`, `medium`, `low`, `info`).
    pub severity: String,
    /// Broad vulnerability category.
    pub category: String,
    /// Human-readable description of the vulnerability.
    pub description: String,
    /// Actionable recommendation for the developer.
    pub recommendation: String,
    /// Path of the source file in which the match was found.
    pub file: String,
    /// 1-based line number of the match.
    pub line: usize,
    /// Source-code snippet around the match.
    pub snippet: String,
}

/// Scan `source` against every entry in `vulns` and return all matches.
///
/// Each [`VulnEntry`] whose `pattern` regex matches anywhere in `source`
/// produces one [`VulnMatch`] per occurrence.  Invalid regex patterns are
/// silently skipped (they are already validated at database load time via
/// [`super::VulnDatabase::validate`]).
pub fn scan_source(vulns: &[VulnEntry], source: &str, file_name: &str) -> Vec<VulnMatch> {
    let mut matches = Vec::new();

    for vuln in vulns {
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
