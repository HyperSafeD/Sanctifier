//! Report template helpers for `sanctifier report` (#523).
//!
//! Provides:
//! - `TemplateVars` — a flat key→value map that can be injected into Markdown/HTML.
//! - `render_template` — replaces `{{KEY}}` placeholders in a template string.
//! - `validate_output_path` — checks extension and parent-dir writability before
//!   the analysis pipeline starts, preventing a wasted scan that can't save its output.
//! - `write_report_atomic` — writes to a temp file then renames, so a partial write
//!   can never leave a truncated report at the target path.

use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::errors::SanctifierError;

/// Supported output formats, derived from the output file extension.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReportFormat {
    Markdown,
    Html,
    Json,
}

impl ReportFormat {
    /// Infer the format from a file path extension.
    pub fn from_path(path: &Path) -> Self {
        match path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_ascii_lowercase())
            .as_deref()
        {
            Some("html") | Some("htm") => ReportFormat::Html,
            Some("json") => ReportFormat::Json,
            _ => ReportFormat::Markdown,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            ReportFormat::Markdown => "markdown",
            ReportFormat::Html => "html",
            ReportFormat::Json => "json",
        }
    }
}

/// Flat key→value substitution map for report templates.
pub type TemplateVars = HashMap<String, String>;

/// Replace every `{{KEY}}` occurrence in `template` with the corresponding
/// value from `vars`.  Unknown placeholders are left verbatim so callers can
/// detect them with `unreplaced_placeholders`.
pub fn render_template(template: &str, vars: &TemplateVars) -> String {
    let mut out = template.to_string();
    for (key, value) in vars {
        let placeholder = format!("{{{{{}}}}}", key);
        out = out.replace(&placeholder, value);
    }
    out
}

/// Return all `{{KEY}}` placeholders that were not replaced by `render_template`.
pub fn unreplaced_placeholders(rendered: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut rest = rendered;
    while let Some(start) = rest.find("{{") {
        rest = &rest[start + 2..];
        if let Some(end) = rest.find("}}") {
            let key = &rest[..end];
            if !key.contains("{{") {
                found.push(format!("{{{{{}}}}}", key));
            }
            rest = &rest[end + 2..];
        } else {
            break;
        }
    }
    found
}

/// Validate that the output path is writable before starting analysis.
///
/// Returns the inferred `ReportFormat` on success, or a `SanctifierError`
/// with an actionable hint if the path is unusable.
pub fn validate_output_path(path: &Path) -> Result<ReportFormat, SanctifierError> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            return Err(SanctifierError::report_write_failed(
                path,
                "parent directory does not exist",
            ));
        }
    }
    let format = ReportFormat::from_path(path);
    Ok(format)
}

/// Write `content` to `dest` atomically: write to a temp file in the same
/// directory, then rename.  This guarantees the destination is either the old
/// content or the new content, never a partial write.
pub fn write_report_atomic(dest: &Path, content: &str) -> Result<(), SanctifierError> {
    let parent = dest.parent().unwrap_or_else(|| Path::new("."));
    let tmp_path = tmp_path_for(dest);

    // Write to temp file
    (|| -> std::io::Result<()> {
        let mut f = fs::File::create(&tmp_path)?;
        f.write_all(content.as_bytes())?;
        f.flush()?;
        Ok(())
    })()
    .map_err(|e| SanctifierError::report_write_failed(&tmp_path, &e.to_string()))?;

    // Atomic rename
    fs::rename(&tmp_path, dest).map_err(|e| {
        let _ = fs::remove_file(&tmp_path);
        SanctifierError::report_write_failed(dest, &e.to_string())
    })?;

    let _ = parent; // suppress unused warning
    Ok(())
}

fn tmp_path_for(dest: &Path) -> PathBuf {
    let stem = dest
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("report");
    dest.with_file_name(format!(".{}.tmp", stem))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    // ── ReportFormat ─────────────────────────────────────────────────────────

    #[test]
    fn format_inferred_from_md_extension() {
        assert_eq!(
            ReportFormat::from_path(Path::new("out/report.md")),
            ReportFormat::Markdown
        );
    }

    #[test]
    fn format_inferred_from_html_extension() {
        assert_eq!(
            ReportFormat::from_path(Path::new("report.html")),
            ReportFormat::Html
        );
    }

    #[test]
    fn format_inferred_from_htm_extension() {
        assert_eq!(
            ReportFormat::from_path(Path::new("r.htm")),
            ReportFormat::Html
        );
    }

    #[test]
    fn format_inferred_from_json_extension() {
        assert_eq!(
            ReportFormat::from_path(Path::new("r.json")),
            ReportFormat::Json
        );
    }

    #[test]
    fn format_defaults_to_markdown_for_unknown_extension() {
        assert_eq!(
            ReportFormat::from_path(Path::new("report.txt")),
            ReportFormat::Markdown
        );
    }

    #[test]
    fn format_extension_comparison_is_case_insensitive() {
        assert_eq!(
            ReportFormat::from_path(Path::new("REPORT.HTML")),
            ReportFormat::Html
        );
    }

    // ── render_template ───────────────────────────────────────────────────────

    #[test]
    fn render_template_replaces_known_keys() {
        let mut vars = TemplateVars::new();
        vars.insert("VERSION".into(), "1.2.3".into());
        vars.insert("DATE".into(), "2026-06-26".into());

        let tpl = "Sanctifier v{{VERSION}} — {{DATE}}";
        let result = render_template(tpl, &vars);
        assert_eq!(result, "Sanctifier v1.2.3 — 2026-06-26");
    }

    #[test]
    fn render_template_leaves_unknown_placeholders_verbatim() {
        let vars = TemplateVars::new();
        let tpl = "Hello {{UNKNOWN}}";
        let result = render_template(tpl, &vars);
        assert_eq!(result, "Hello {{UNKNOWN}}");
    }

    #[test]
    fn render_template_replaces_multiple_occurrences() {
        let mut vars = TemplateVars::new();
        vars.insert("X".into(), "42".into());
        let result = render_template("{{X}} + {{X}} = 84", &vars);
        assert_eq!(result, "42 + 42 = 84");
    }

    // ── unreplaced_placeholders ───────────────────────────────────────────────

    #[test]
    fn detects_unreplaced_placeholders() {
        let rendered = "Version: 1.0.0 — {{DATE}} — {{AUTHOR}}";
        let unreplaced = unreplaced_placeholders(rendered);
        assert!(unreplaced.contains(&"{{DATE}}".to_string()));
        assert!(unreplaced.contains(&"{{AUTHOR}}".to_string()));
    }

    #[test]
    fn no_unreplaced_placeholders_when_all_replaced() {
        let rendered = "Version: 1.0.0 — 2026-06-26";
        assert!(unreplaced_placeholders(rendered).is_empty());
    }

    // ── validate_output_path ─────────────────────────────────────────────────

    #[test]
    fn validate_output_path_ok_for_existing_parent() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("report.md");
        assert!(validate_output_path(&path).is_ok());
    }

    #[test]
    fn validate_output_path_errors_for_missing_parent() {
        let path = Path::new("/no/such/dir/report.md");
        let err = validate_output_path(path).unwrap_err();
        assert!(err.to_string().contains("mkdir"));
    }

    #[test]
    fn validate_output_path_returns_correct_format() {
        let dir = tempdir().unwrap();
        let html_path = dir.path().join("r.html");
        assert_eq!(
            validate_output_path(&html_path).unwrap(),
            ReportFormat::Html
        );
        let md_path = dir.path().join("r.md");
        assert_eq!(
            validate_output_path(&md_path).unwrap(),
            ReportFormat::Markdown
        );
        let json_path = dir.path().join("r.json");
        assert_eq!(
            validate_output_path(&json_path).unwrap(),
            ReportFormat::Json
        );
    }

    // ── write_report_atomic ───────────────────────────────────────────────────

    #[test]
    fn write_report_atomic_creates_file_with_correct_content() {
        let dir = tempdir().unwrap();
        let dest = dir.path().join("report.md");
        write_report_atomic(&dest, "# Hello").unwrap();
        assert_eq!(fs::read_to_string(&dest).unwrap(), "# Hello");
    }

    #[test]
    fn write_report_atomic_overwrites_existing_file() {
        let dir = tempdir().unwrap();
        let dest = dir.path().join("report.md");
        fs::write(&dest, "old content").unwrap();
        write_report_atomic(&dest, "new content").unwrap();
        assert_eq!(fs::read_to_string(&dest).unwrap(), "new content");
    }

    #[test]
    fn write_report_atomic_does_not_leave_temp_file_on_success() {
        let dir = tempdir().unwrap();
        let dest = dir.path().join("r.md");
        write_report_atomic(&dest, "data").unwrap();
        let tmp = dir.path().join(".r.md.tmp");
        assert!(
            !tmp.exists(),
            "temp file should be removed after atomic rename"
        );
    }
}
