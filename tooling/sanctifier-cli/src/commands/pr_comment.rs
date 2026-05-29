//! # GitHub PR Comment Formatter
//!
//! Renders a structured "delta vs base branch" summary for posting as a
//! sticky GitHub PR comment. Produces:
//!
//! - `+N findings` (new), `-M findings` (resolved), severity breakdown
//! - A link to the full SARIF/JSON report artifact
//! - Markdown table of new findings grouped by severity

use crate::commands::analyze::SeverityLevel;
use serde_json::Value;
use std::collections::HashMap;

/// Summary of changes between base and head scan.
#[derive(Debug, Default)]
pub struct DiffSummary {
    pub new_count: usize,
    pub resolved_count: usize,
    /// new findings grouped by severity label
    pub by_severity: HashMap<String, usize>,
    /// top new findings (up to 10) for inline display
    pub top_new: Vec<FindingSummary>,
}

#[derive(Debug)]
pub struct FindingSummary {
    pub rule: String,
    pub severity: String,
    pub file: String,
    pub message: String,
}

/// Build a [`DiffSummary`] from the JSON diff output produced by `sanctifier diff`.
pub fn build_diff_summary(diff_json: &Value) -> DiffSummary {
    let mut summary = DiffSummary::default();

    // New findings
    if let Some(new) = diff_json.get("new_findings") {
        let files = new.as_object().map(|o| o.values().collect::<Vec<_>>()).unwrap_or_default();
        for file_val in &files {
            if let Some(findings) = file_val.get("findings").and_then(|f| f.as_array()) {
                for f in findings {
                    summary.new_count += 1;
                    let sev = f.get("severity")
                        .and_then(|s| s.as_str())
                        .unwrap_or("unknown")
                        .to_string();
                    *summary.by_severity.entry(sev.clone()).or_insert(0) += 1;
                    if summary.top_new.len() < 10 {
                        summary.top_new.push(FindingSummary {
                            rule: f.get("rule_id").and_then(|v| v.as_str()).unwrap_or("unknown").to_string(),
                            severity: sev,
                            file: file_val.get("file").and_then(|v| v.as_str()).unwrap_or("?").to_string(),
                            message: f.get("message").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                        });
                    }
                }
            }
        }
    }

    // Resolved findings (present in baseline but not in current)
    if let Some(resolved) = diff_json.get("resolved_findings_count").and_then(|v| v.as_u64()) {
        summary.resolved_count = resolved as usize;
    }

    summary
}

/// Render the diff summary as a GitHub-flavoured Markdown PR comment.
///
/// The comment starts with a unique HTML comment marker so the GitHub Action
/// can find and edit it (sticky comment pattern).
pub fn render_pr_comment(summary: &DiffSummary, artifact_url: Option<&str>) -> String {
    let mut md = String::new();

    // Sticky marker — used by the Action to find and update the comment
    md.push_str("<!-- sanctifier-pr-comment -->
");
    md.push_str("## 🛡️ Sanctifier Security Analysis

");

    // Delta line
    let new_str = if summary.new_count > 0 {
        format!("`+{}` new finding{}", summary.new_count, if summary.new_count == 1 { "" } else { "s" })
    } else {
        "`+0` new findings".to_string()
    };
    let resolved_str = if summary.resolved_count > 0 {
        format!("`-{}` resolved", summary.resolved_count)
    } else {
        "`-0` resolved".to_string()
    };
    md.push_str(&format!("{new_str} · {resolved_str}

"));

    // Severity breakdown
    if !summary.by_severity.is_empty() {
        let order = ["critical", "high", "medium", "low", "info"];
        let parts: Vec<String> = order.iter()
            .filter_map(|sev| summary.by_severity.get(*sev).map(|n| format!("**{}** {}", sev, n)))
            .collect();
        if !parts.is_empty() {
            md.push_str(&format!("Severity: {}

", parts.join(" · ")));
        }
    }

    // Top new findings table
    if !summary.top_new.is_empty() {
        md.push_str("<details>
<summary>New findings</summary>

");
        md.push_str("| Severity | Rule | File | Message |
");
        md.push_str("|---|---|---|---|
");
        for f in &summary.top_new {
            let msg = f.message.chars().take(80).collect::<String>();
            md.push_str(&format!("| {} | `{}` | `{}` | {} |
", f.severity, f.rule, f.file, msg));
        }
        md.push_str("
</details>

");
    } else if summary.new_count == 0 {
        md.push_str("✅ No new findings — great work!

");
    }

    // Artifact link
    if let Some(url) = artifact_url {
        md.push_str(&format!("[📄 Full report]({url})
"));
    }

    md
}

/// Severity order for comparison (higher = more severe).
pub fn severity_rank(s: &str) -> u8 {
    match s.to_lowercase().as_str() {
        "critical" => 4,
        "high"     => 3,
        "medium"   => 2,
        "low"      => 1,
        _          => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn make_diff_json(new_count: usize, resolved: usize) -> Value {
        let findings: Vec<Value> = (0..new_count).map(|i| json!({
            "rule_id": format!("rule_{i}"),
            "severity": if i % 2 == 0 { "high" } else { "medium" },
            "message": format!("finding {i}"),
        })).collect();
        json!({
            "new_findings": {
                "src/lib.rs": { "file": "src/lib.rs", "findings": findings }
            },
            "resolved_findings_count": resolved,
        })
    }

    #[test]
    fn test_build_summary_counts() {
        let diff = make_diff_json(4, 2);
        let s = build_diff_summary(&diff);
        assert_eq!(s.new_count, 4);
        assert_eq!(s.resolved_count, 2);
        assert_eq!(s.by_severity.get("high").copied().unwrap_or(0), 2);
        assert_eq!(s.by_severity.get("medium").copied().unwrap_or(0), 2);
    }

    #[test]
    fn test_render_contains_marker() {
        let diff = make_diff_json(2, 1);
        let s = build_diff_summary(&diff);
        let md = render_pr_comment(&s, Some("https://example.com/report"));
        assert!(md.contains("<!-- sanctifier-pr-comment -->"));
        assert!(md.contains("+2"));
        assert!(md.contains("-1"));
        assert!(md.contains("https://example.com/report"));
    }

    #[test]
    fn test_render_no_findings_shows_checkmark() {
        let diff = make_diff_json(0, 0);
        let s = build_diff_summary(&diff);
        let md = render_pr_comment(&s, None);
        assert!(md.contains("No new findings"));
    }

    #[test]
    fn test_severity_rank_order() {
        assert!(severity_rank("critical") > severity_rank("high"));
        assert!(severity_rank("high") > severity_rank("medium"));
        assert!(severity_rank("medium") > severity_rank("low"));
    }
}
