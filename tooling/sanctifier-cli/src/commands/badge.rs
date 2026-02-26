use anyhow::Context;
use clap::Args;
use serde::Deserialize;
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
        .unwrap_or_else(|| normalize_path_for_markdown(&args.svg_output));
    let markdown = format!("![Sanctifier: {}]({})", status.text(), markdown_url);

    if let Some(md_path) = args.markdown_output {
        write_text_file(&md_path, &(markdown.clone() + "\n"))?;
        println!("Markdown snippet written to {}", md_path.display());
    } else {
        println!("{}", markdown);
    }

    println!("Badge generated at {}", args.svg_output.display());
    println!("Current status: {}", status.text());
    Ok(())
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
        };
        exec(args).expect("badge command should succeed");

        let svg = fs::read_to_string(svg_path).expect("svg should exist");
        let md = fs::read_to_string(md_path).expect("markdown should exist");

        assert!(svg.contains("Sanctifier"));
        assert!(svg.contains("Secure"));
        assert!(md.contains("https://example.com/sanctifier-security.svg"));
    }
}
