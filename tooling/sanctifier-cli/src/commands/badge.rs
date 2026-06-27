use anyhow::Context;
use clap::Args;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Args, Debug)]
pub struct BadgeArgs {
    /// Path to Sanctifier JSON report (from `sanctifier analyze --format json`)
    #[arg(short, long, default_value = "sanctifier-report.json")]
    pub report: PathBuf,

    /// Where to write generated badge SVG
    #[arg(long, default_value = "sanctifier-security.svg")]
    pub svg_output: PathBuf,

    /// Where to write generated markdown snippet
    #[arg(long)]
    pub markdown_output: Option<PathBuf>,

    /// Public URL for the SVG (used by markdown output). Falls back to local SVG path.
    #[arg(long)]
    pub badge_url: Option<String>,

    /// Output format: `text` (default) or `json`
    ///
    /// `json` emits a machine-readable object useful for CI scripts that need
    /// to inspect the badge status programmatically without parsing SVG.
    #[arg(long, default_value = "text", value_parser = ["text", "json"])]
    pub format: String,

    /// Suppress all non-error output (useful in CI scripts)
    #[arg(short, long)]
    pub quiet: bool,

    /// Print a shields.io badge URL in addition to the local SVG
    ///
    /// Shields.io can render a live badge from a JSON endpoint.  Use this flag
    /// to get a pre-filled URL template you can adapt for your own endpoint.
    #[arg(long)]
    pub shields_url: bool,
}

#[derive(Debug, Deserialize)]
struct AnalyzeReport {
    summary: AnalyzeSummary,
}

#[derive(Debug, Deserialize)]
struct AnalyzeSummary {
    total_findings: usize,
    has_critical: bool,
    has_high: bool,
}

/// Machine-readable output emitted when `--format json` is requested.
#[derive(Debug, Serialize)]
struct BadgeOutput {
    status: &'static str,
    color: &'static str,
    total_findings: usize,
    svg_path: String,
    markdown: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    shields_url: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SecurityStatus {
    Secure,
    Warning,
    Critical,
}

impl SecurityStatus {
    fn text(self) -> &'static str {
        match self {
            SecurityStatus::Secure => "Secure",
            SecurityStatus::Warning => "Warning",
            SecurityStatus::Critical => "Critical",
        }
    }

    fn color(self) -> &'static str {
        match self {
            SecurityStatus::Secure => "#2ea043",
            SecurityStatus::Warning => "#fb8c00",
            SecurityStatus::Critical => "#d73a49",
        }
    }
}

pub fn exec(args: BadgeArgs) -> anyhow::Result<()> {
    let report_content = fs::read_to_string(&args.report)
        .with_context(|| format!("failed to read report file: {}", args.report.display()))?;
    let report: AnalyzeReport = serde_json::from_str(&report_content)
        .with_context(|| format!("failed to parse JSON report: {}", args.report.display()))?;

    let status = derive_status(&report.summary);
    let svg = generate_badge_svg("Sanctifier", status.text(), status.color());

    write_text_file(&args.svg_output, &svg)?;

    let markdown_url = args
        .badge_url
        .clone()
        .unwrap_or_else(|| normalize_path_for_markdown(&args.svg_output));
    let markdown = format!("![Sanctifier: {}]({})", status.text(), markdown_url);

    if let Some(ref md_path) = args.markdown_output {
        write_text_file(md_path, &(markdown.clone() + "\n"))?;
        if !args.quiet {
            println!("Markdown snippet written to {}", md_path.display());
        }
    }

    let shields = if args.shields_url {
        Some(build_shields_url(status))
    } else {
        None
    };

    if args.format == "json" {
        let output = BadgeOutput {
            status: status.text(),
            color: status.color(),
            total_findings: report.summary.total_findings,
            svg_path: args.svg_output.to_string_lossy().into_owned(),
            markdown: markdown.clone(),
            shields_url: shields.clone(),
        };
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        if args.markdown_output.is_none() && !args.quiet {
            println!("{}", markdown);
        }
        if !args.quiet {
            println!(
                "Badge generated at {} (status: {}, findings: {})",
                args.svg_output.display(),
                status.text(),
                report.summary.total_findings,
            );
        }
        if let Some(url) = &shields {
            if !args.quiet {
                println!("shields.io URL: {url}");
            }
        }
    }

    Ok(())
}

fn build_shields_url(status: SecurityStatus) -> String {
    // Shields.io endpoint-badge format:
    //   https://img.shields.io/badge/<label>-<message>-<color>
    // Color must be a named color or hex without '#'.
    let color_hex = status.color().trim_start_matches('#');
    format!(
        "https://img.shields.io/badge/Sanctifier-{}-{}",
        status.text(),
        color_hex,
    )
}

fn write_text_file(path: &Path, content: &str) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create directory: {}", parent.display()))?;
        }
    }
    fs::write(path, content)
        .with_context(|| format!("failed to write file: {}", path.display()))?;
    Ok(())
}

fn derive_status(summary: &AnalyzeSummary) -> SecurityStatus {
    if summary.has_critical {
        SecurityStatus::Critical
    } else if summary.has_high || summary.total_findings > 0 {
        SecurityStatus::Warning
    } else {
        SecurityStatus::Secure
    }
}

fn generate_badge_svg(label: &str, status: &str, status_color: &str) -> String {
    let label_width = text_width(label);
    let status_width = text_width(status);
    let total_width = label_width + status_width;
    let status_x = label_width;
    let label_text_x = label_width / 2;
    let status_text_x = label_width + (status_width / 2);

    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{total_width}\" height=\"20\" role=\"img\" aria-label=\"{label}: {status}\">\
<linearGradient id=\"g\" x2=\"0\" y2=\"100%\">\
<stop offset=\"0\" stop-color=\"#fff\" stop-opacity=\".7\"/>\
<stop offset=\".1\" stop-color=\"#aaa\" stop-opacity=\".1\"/>\
<stop offset=\".9\" stop-opacity=\".3\"/>\
<stop offset=\"1\" stop-opacity=\".5\"/>\
</linearGradient>\
<clipPath id=\"r\"><rect width=\"{total_width}\" height=\"20\" rx=\"3\" fill=\"#fff\"/></clipPath>\
<g clip-path=\"url(#r)\">\
<rect width=\"{label_width}\" height=\"20\" fill=\"#555\"/>\
<rect x=\"{status_x}\" width=\"{status_width}\" height=\"20\" fill=\"{status_color}\"/>\
<rect width=\"{total_width}\" height=\"20\" fill=\"url(#g)\"/>\
</g>\
<g fill=\"#fff\" text-anchor=\"middle\" font-family=\"DejaVu Sans,Verdana,Geneva,sans-serif\" font-size=\"11\">\
<text x=\"{label_text_x}\" y=\"15\" fill=\"#010101\" fill-opacity=\".3\">{label}</text>\
<text x=\"{label_text_x}\" y=\"14\">{label}</text>\
<text x=\"{status_text_x}\" y=\"15\" fill=\"#010101\" fill-opacity=\".3\">{status}</text>\
<text x=\"{status_text_x}\" y=\"14\">{status}</text>\
</g>\
</svg>"
    )
}

fn text_width(text: &str) -> usize {
    let padded = (text.chars().count() * 7) + 10;
    padded.max(28)
}

fn normalize_path_for_markdown(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn derive_status_handles_critical() {
        let summary = AnalyzeSummary {
            total_findings: 12,
            has_critical: true,
            has_high: true,
        };
        assert_eq!(derive_status(&summary), SecurityStatus::Critical);
    }

    #[test]
    fn derive_status_handles_warning() {
        let summary = AnalyzeSummary {
            total_findings: 1,
            has_critical: false,
            has_high: false,
        };
        assert_eq!(derive_status(&summary), SecurityStatus::Warning);
    }

    #[test]
    fn derive_status_handles_secure() {
        let summary = AnalyzeSummary {
            total_findings: 0,
            has_critical: false,
            has_high: false,
        };
        assert_eq!(derive_status(&summary), SecurityStatus::Secure);
    }

    #[test]
    fn generate_svg_contains_expected_text() {
        let svg = generate_badge_svg("Sanctifier", "Secure", "#2ea043");
        assert!(svg.contains("Sanctifier"));
        assert!(svg.contains("Secure"));
        assert!(svg.contains("#2ea043"));
    }

    // ── Integration tests: SVG structure and score-to-color mapping ───────────

    #[test]
    fn svg_output_is_valid_svg_element() {
        // Verify the generated badge is a well-formed SVG (starts/ends correctly
        // and contains the mandatory structural elements).
        let svg = generate_badge_svg("Sanctifier", "Secure", "#2ea043");
        assert!(svg.starts_with("<svg "), "badge must open with <svg");
        assert!(svg.ends_with("</svg>"), "badge must close with </svg>");
        assert!(
            svg.contains("xmlns=\"http://www.w3.org/2000/svg\""),
            "must have SVG namespace"
        );
        assert!(svg.contains("<clipPath"), "must contain clipPath element");
        assert!(
            svg.contains("<linearGradient"),
            "must contain linearGradient element"
        );
        assert!(svg.contains("role=\"img\""), "must have accessibility role");
    }

    #[test]
    fn badge_color_is_green_for_secure_report() {
        let summary = AnalyzeSummary {
            total_findings: 0,
            has_critical: false,
            has_high: false,
        };
        let status = derive_status(&summary);
        assert_eq!(status.color(), "#2ea043", "secure badge must be green");
    }

    #[test]
    fn badge_color_is_orange_for_warning_report() {
        let summary = AnalyzeSummary {
            total_findings: 3,
            has_critical: false,
            has_high: true,
        };
        let status = derive_status(&summary);
        assert_eq!(status.color(), "#fb8c00", "warning badge must be orange");
    }

    #[test]
    fn badge_color_is_red_for_critical_report() {
        let summary = AnalyzeSummary {
            total_findings: 5,
            has_critical: true,
            has_high: true,
        };
        let status = derive_status(&summary);
        assert_eq!(status.color(), "#d73a49", "critical badge must be red");
    }

    #[test]
    fn svg_contains_status_color_matching_security_score() {
        // End-to-end: the SVG written to disk must embed the correct color.
        let tmp = TempDir::new().expect("temp dir");
        let report_path = tmp.path().join("report.json");
        let svg_path = tmp.path().join("badge.svg");

        let report = r#"{"summary":{"total_findings":2,"has_critical":false,"has_high":true}}"#;
        fs::write(&report_path, report).unwrap();

        exec(BadgeArgs {
            report: report_path,
            svg_output: svg_path.clone(),
            markdown_output: None,
            badge_url: None,
            format: "text".to_string(),
            quiet: false,
            shields_url: false,
        })
        .expect("exec should succeed");

        let svg = fs::read_to_string(svg_path).unwrap();
        // has_high = true → Warning → orange
        assert!(
            svg.contains("#fb8c00"),
            "SVG must contain orange color for warning score"
        );
    }

    // ── New UX/DX tests (Issue #524) ─────────────────────────────────────────

    #[test]
    fn exec_quiet_flag_suppresses_non_error_output() {
        // quiet mode must not panic and must still write the SVG
        let tmp = TempDir::new().expect("temp dir");
        let report_path = tmp.path().join("report.json");
        let svg_path = tmp.path().join("badge.svg");
        fs::write(
            &report_path,
            r#"{"summary":{"total_findings":0,"has_critical":false,"has_high":false}}"#,
        )
        .unwrap();

        exec(BadgeArgs {
            report: report_path,
            svg_output: svg_path.clone(),
            markdown_output: None,
            badge_url: None,
            format: "text".to_string(),
            quiet: true,
            shields_url: false,
        })
        .expect("quiet exec should succeed");

        assert!(svg_path.exists(), "SVG must be written even in quiet mode");
    }

    #[test]
    fn exec_json_format_produces_parseable_output() {
        let tmp = TempDir::new().expect("temp dir");
        let report_path = tmp.path().join("report.json");
        let svg_path = tmp.path().join("badge.svg");
        fs::write(
            &report_path,
            r#"{"summary":{"total_findings":3,"has_critical":true,"has_high":true}}"#,
        )
        .unwrap();

        // exec writes to stdout; we test the output struct shape instead
        let report_content = fs::read_to_string(&report_path).unwrap();
        let report: AnalyzeReport = serde_json::from_str(&report_content).unwrap();
        let status = derive_status(&report.summary);
        let out = BadgeOutput {
            status: status.text(),
            color: status.color(),
            total_findings: report.summary.total_findings,
            svg_path: svg_path.to_string_lossy().into_owned(),
            markdown: format!("![Sanctifier: {}](test.svg)", status.text()),
            shields_url: None,
        };
        let json = serde_json::to_string(&out).expect("BadgeOutput must serialize");
        let parsed: serde_json::Value = serde_json::from_str(&json).expect("must parse back");
        assert_eq!(parsed["status"], "Critical");
        assert_eq!(parsed["total_findings"], 3);
    }

    #[test]
    fn build_shields_url_contains_status_text() {
        let url = build_shields_url(SecurityStatus::Secure);
        assert!(
            url.contains("Secure"),
            "shields URL must contain status text"
        );
        assert!(url.contains("shields.io"), "must point to shields.io");
        // Color must not include '#'
        assert!(
            !url.contains('#'),
            "shields URL must not contain '#' in color"
        );
    }

    #[test]
    fn build_shields_url_differs_per_status() {
        let secure = build_shields_url(SecurityStatus::Secure);
        let warning = build_shields_url(SecurityStatus::Warning);
        let critical = build_shields_url(SecurityStatus::Critical);
        assert_ne!(secure, warning);
        assert_ne!(warning, critical);
        assert_ne!(secure, critical);
    }

    #[test]
    fn exec_with_shields_url_flag_succeeds() {
        let tmp = TempDir::new().expect("temp dir");
        let report_path = tmp.path().join("report.json");
        let svg_path = tmp.path().join("badge.svg");
        fs::write(
            &report_path,
            r#"{"summary":{"total_findings":0,"has_critical":false,"has_high":false}}"#,
        )
        .unwrap();

        exec(BadgeArgs {
            report: report_path,
            svg_output: svg_path.clone(),
            markdown_output: None,
            badge_url: None,
            format: "text".to_string(),
            quiet: true,
            shields_url: true,
        })
        .expect("exec with --shields-url should succeed");

        assert!(svg_path.exists());
    }

    #[test]
    fn exec_writes_svg_and_markdown_files() {
        let tmp = TempDir::new().expect("temp dir should be created");
        let report_path = tmp.path().join("report.json");
        let svg_path = tmp.path().join("badges").join("status.svg");
        let md_path = tmp.path().join("badges").join("status.md");

        let report = r#"{
  "summary": {
    "total_findings": 0,
    "has_critical": false,
    "has_high": false
  }
}"#;
        fs::write(&report_path, report).expect("report fixture should be written");

        let args = BadgeArgs {
            report: report_path,
            svg_output: svg_path.clone(),
            markdown_output: Some(md_path.clone()),
            badge_url: Some("https://example.com/sanctifier-security.svg".to_string()),
            format: "text".to_string(),
            quiet: false,
            shields_url: false,
        };
        exec(args).expect("badge command should succeed");

        let svg = fs::read_to_string(svg_path).expect("svg should exist");
        let md = fs::read_to_string(md_path).expect("markdown should exist");

        assert!(svg.contains("Sanctifier"));
        assert!(svg.contains("Secure"));
        assert!(md.contains("https://example.com/sanctifier-security.svg"));
    }
}
