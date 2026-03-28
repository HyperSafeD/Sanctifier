//! `sanctifier report` — generate a Markdown or HTML security report.
//!
//! Runs the same analysis pipeline as `sanctifier analyze` but formats the
//! output as a human-readable document rather than plain text or JSON.
//!
//! # Format selection
//! - `--output report.md`   → Markdown (default when no extension or `.md`)
//! - `--output report.html` → HTML
//! - No `--output`          → Markdown printed to stdout

use crate::commands::analyze::{
    analyze_single_file, collect_rs_files, is_soroban_project, load_config, run_with_timeout,
    FileAnalysisResult,
};
use crate::vulndb::{VulnDatabase, VulnMatch};
use clap::Args;
use rayon::prelude::*;
use sanctifier_core::{Analyzer, SizeWarningLevel};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tracing::warn;

// ── CLI arguments ─────────────────────────────────────────────────────────────

#[derive(Args, Debug)]
pub struct ReportArgs {
    /// Path to the contract directory or a single `.rs` file
    #[arg(default_value = ".")]
    pub path: PathBuf,

    /// Write report to this file (`.md` → Markdown, `.html` → HTML).
    /// Prints Markdown to stdout when omitted.
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// Ledger entry size limit in bytes
    #[arg(short, long, default_value = "64000")]
    pub limit: usize,

    /// Path to a custom vulnerability database JSON file
    #[arg(long)]
    pub vuln_db: Option<PathBuf>,

    /// Per-file analysis timeout in seconds (0 = disabled)
    #[arg(short, long, default_value = "30")]
    pub timeout: u64,
}

// ── Entry point ───────────────────────────────────────────────────────────────

pub fn exec(args: ReportArgs) -> anyhow::Result<()> {
    let path = &args.path;

    if !is_soroban_project(path) {
        anyhow::bail!(
            "{:?} is not a valid Soroban project (no Cargo.toml with soroban-sdk found)",
            path
        );
    }

    // Load config + apply CLI overrides
    let mut config = load_config(path);
    config.ledger_limit = args.limit;
    let analyzer = Arc::new(Analyzer::new(config));

    // Vulnerability database
    let vuln_db = Arc::new(match &args.vuln_db {
        Some(db_path) => VulnDatabase::load(db_path)?,
        None => VulnDatabase::load_default(),
    });

    // Collect .rs files
    let rs_files = if path.is_dir() {
        collect_rs_files(path, &analyzer.config.ignore_paths)
    } else if path.extension().and_then(|s| s.to_str()) == Some("rs") {
        vec![path.clone()]
    } else {
        vec![]
    };

    let total_files = rs_files.len();
    let counter = Arc::new(AtomicUsize::new(0));
    let timeout_dur = if args.timeout == 0 {
        None
    } else {
        Some(Duration::from_secs(args.timeout))
    };

    // Parallel analysis
    let mut results: Vec<FileAnalysisResult> = rs_files
        .par_iter()
        .map(|file_path| {
            let idx = counter.fetch_add(1, Ordering::Relaxed) + 1;
            let file_name = file_path.display().to_string();
            eprintln!("[{}/{}] Analyzing {}", idx, total_files, file_name);

            let content = match fs::read_to_string(file_path) {
                Ok(c) => c,
                Err(_) => return FileAnalysisResult::default(),
            };

            let analyzer = Arc::clone(&analyzer);
            let vuln_db = Arc::clone(&vuln_db);
            let file_name_clone = file_name.clone();

            match run_with_timeout(timeout_dur, move || {
                analyze_single_file(&analyzer, &vuln_db, &content, &file_name_clone)
            }) {
                Some(res) => res,
                None => {
                    warn!(
                        target: "sanctifier",
                        file = %file_name,
                        timeout_secs = args.timeout,
                        "Analysis timed out"
                    );
                    FileAnalysisResult {
                        file_path: file_name,
                        timed_out: true,
                        ..Default::default()
                    }
                }
            }
        })
        .collect();

    results.sort_by(|a, b| a.file_path.cmp(&b.file_path));

    // Merge
    let data = merge_results(results);

    // Determine output format
    let is_html = args
        .output
        .as_ref()
        .and_then(|p| p.extension())
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("html"))
        .unwrap_or(false);

    let report_text = if is_html {
        render_html(&data, path, &vuln_db.version)
    } else {
        render_markdown(&data, path, &vuln_db.version)
    };

    match &args.output {
        Some(out_path) => {
            fs::write(out_path, &report_text)?;
            println!("Report written to {}", out_path.display());
        }
        None => print!("{}", report_text),
    }

    Ok(())
}

// ── Merged analysis data ──────────────────────────────────────────────────────

struct ReportData {
    auth_gaps: Vec<sanctifier_core::AuthGapIssue>,
    panic_issues: Vec<sanctifier_core::PanicIssue>,
    arithmetic_issues: Vec<sanctifier_core::ArithmeticIssue>,
    size_warnings: Vec<sanctifier_core::SizeWarning>,
    unsafe_patterns: Vec<sanctifier_core::UnsafePattern>,
    collisions: Vec<sanctifier_core::StorageCollisionIssue>,
    event_issues: Vec<sanctifier_core::EventIssue>,
    unhandled_results: Vec<sanctifier_core::UnhandledResultIssue>,
    upgrade_findings: Vec<sanctifier_core::UpgradeFinding>,
    smt_issues: Vec<sanctifier_core::smt::SmtInvariantIssue>,
    sep41_issues: Vec<sanctifier_core::Sep41Issue>,
    vuln_matches: Vec<VulnMatch>,
    timed_out_files: Vec<String>,
    has_critical: bool,
    has_high: bool,
}

fn merge_results(results: Vec<FileAnalysisResult>) -> ReportData {
    let mut data = ReportData {
        auth_gaps: vec![],
        panic_issues: vec![],
        arithmetic_issues: vec![],
        size_warnings: vec![],
        unsafe_patterns: vec![],
        collisions: vec![],
        event_issues: vec![],
        unhandled_results: vec![],
        upgrade_findings: vec![],
        smt_issues: vec![],
        sep41_issues: vec![],
        vuln_matches: vec![],
        timed_out_files: vec![],
        has_critical: false,
        has_high: false,
    };

    for r in results {
        if r.timed_out {
            data.timed_out_files.push(r.file_path.clone());
        }
        data.auth_gaps.extend(r.auth_gaps);
        data.panic_issues.extend(r.panic_issues);
        data.arithmetic_issues.extend(r.arithmetic_issues);
        data.size_warnings.extend(r.size_warnings);
        data.unsafe_patterns.extend(r.unsafe_patterns);
        data.collisions.extend(r.collisions);
        data.event_issues.extend(r.event_issues);
        data.unhandled_results.extend(r.unhandled_results);
        for rep in r.upgrade_reports {
            data.upgrade_findings.extend(rep.findings);
        }
        data.smt_issues.extend(r.smt_issues);
        data.sep41_issues.extend(r.sep41_issues);
        data.vuln_matches.extend(r.vuln_matches);
    }

    data.has_critical =
        !data.auth_gaps.is_empty() || data.panic_issues.iter().any(|p| p.issue_type == "panic!");
    data.has_high = !data.arithmetic_issues.is_empty()
        || !data.panic_issues.is_empty()
        || !data.smt_issues.is_empty()
        || !data.sep41_issues.is_empty()
        || !data.unhandled_results.is_empty()
        || data
            .size_warnings
            .iter()
            .any(|w| w.level == SizeWarningLevel::ExceedsLimit);

    data
}

// ── Markdown renderer ─────────────────────────────────────────────────────────

fn render_markdown(data: &ReportData, path: &Path, vuln_db_version: &str) -> String {
    let mut md = String::new();
    let version = env!("CARGO_PKG_VERSION");
    let date = human_date();

    // ── Header ──
    md.push_str("# Sanctifier Security Report\n\n");
    md.push_str("| | |\n|---|---|\n");
    md.push_str(&format!("| **Contract path** | `{}` |\n", path.display()));
    md.push_str(&format!("| **Analysis date** | {} |\n", date));
    md.push_str(&format!("| **Tool version** | {} |\n", version));
    md.push_str(&format!("| **Vuln DB version** | {} |\n", vuln_db_version));
    let overall = if data.has_critical {
        "🔴 Critical"
    } else if data.has_high {
        "🟠 High"
    } else {
        "🟢 Pass"
    };
    md.push_str(&format!("| **Overall** | {} |\n\n", overall));

    // ── Summary table ──
    md.push_str("## Summary\n\n");
    md.push_str("| Category | Code | Count | Severity |\n");
    md.push_str("|---|---|:---:|---|\n");

    let rows: &[(&str, &str, usize, &str)] = &[
        (
            "Authentication Gaps",
            "S001",
            data.auth_gaps.len(),
            "🔴 Critical",
        ),
        (
            "Panic / Unwrap / Expect",
            "S002",
            data.panic_issues.len(),
            "🔴 High",
        ),
        (
            "Arithmetic Overflow",
            "S003",
            data.arithmetic_issues.len(),
            "🟠 High",
        ),
        (
            "Ledger Size Risk",
            "S004",
            data.size_warnings.len(),
            "🟡 Medium",
        ),
        (
            "Storage Collisions",
            "S005",
            data.collisions.len(),
            "🟠 High",
        ),
        (
            "Unsafe Patterns",
            "S006",
            data.unsafe_patterns.len(),
            "🟡 Medium",
        ),
        (
            "Event Inconsistencies",
            "S008",
            data.event_issues.len(),
            "🔵 Low",
        ),
        (
            "Unhandled Results",
            "S009",
            data.unhandled_results.len(),
            "🟠 High",
        ),
        (
            "Upgrade Risks",
            "S010",
            data.upgrade_findings.len(),
            "🟠 High",
        ),
        (
            "SMT Invariant Violations",
            "S011",
            data.smt_issues.len(),
            "🔴 Critical",
        ),
        (
            "SEP-41 Deviations",
            "S012",
            data.sep41_issues.len(),
            "🟡 Medium",
        ),
        ("Vuln DB Matches", "—", data.vuln_matches.len(), "varies"),
        (
            "Analysis Timeouts",
            "S000",
            data.timed_out_files.len(),
            "ℹ️ Info",
        ),
    ];

    let total: usize = rows.iter().map(|r| r.2).sum();
    for (cat, code, count, sev) in rows {
        md.push_str(&format!("| {} | `{}` | {} | {} |\n", cat, code, count, sev));
    }
    md.push_str(&format!("| **Total** | | **{}** | |\n\n", total));

    // ── Findings ──
    md.push_str("## Findings\n\n");

    if !data.auth_gaps.is_empty() {
        md.push_str("### 🔴 Authentication Gaps (S001)\n\n");
        md.push_str(
            "> Missing `require_auth()` in state-mutating or externally-callable functions.\n\n",
        );
        for g in &data.auth_gaps {
            md.push_str(&format!("- `{}`\n", g.function_name));
        }
        md.push('\n');
    }

    if !data.panic_issues.is_empty() {
        md.push_str("### 🔴 Panic / Unwrap / Expect (S002)\n\n");
        md.push_str(
            "> `panic!`, `.unwrap()`, or `.expect()` may abort the contract execution.\n\n",
        );
        md.push_str("| Function | Type | Location |\n|---|---|---|\n");
        for p in &data.panic_issues {
            md.push_str(&format!(
                "| `{}` | `{}` | `{}` |\n",
                p.function_name, p.issue_type, p.location
            ));
        }
        md.push('\n');
    }

    if !data.arithmetic_issues.is_empty() {
        md.push_str("### 🟠 Arithmetic Overflow / Underflow (S003)\n\n");
        md.push_str("> Unchecked arithmetic can silently wrap or panic in debug mode.\n\n");
        md.push_str("| Function | Operation | Suggestion | Location |\n|---|---|---|---|\n");
        for a in &data.arithmetic_issues {
            md.push_str(&format!(
                "| `{}` | `{}` | {} | `{}` |\n",
                a.function_name, a.operation, a.suggestion, a.location
            ));
        }
        md.push('\n');
    }

    if !data.size_warnings.is_empty() {
        md.push_str("### 🟡 Ledger Size Risk (S004)\n\n");
        md.push_str(
            "> Structs that approach or exceed the ledger entry limit will fail to persist.\n\n",
        );
        md.push_str("| Struct | Estimated Size | Limit | Level |\n|---|---|---|---|\n");
        for w in &data.size_warnings {
            let level = format!("{:?}", w.level);
            md.push_str(&format!(
                "| `{}` | {} B | {} B | {} |\n",
                w.struct_name, w.estimated_size, w.limit, level
            ));
        }
        md.push('\n');
    }

    if !data.collisions.is_empty() {
        md.push_str("### 🟠 Storage Key Collisions (S005)\n\n");
        md.push_str("> Two data paths share a storage key, causing silent overwrites.\n\n");
        md.push_str("| Key | Type | Location | Message |\n|---|---|---|---|\n");
        for c in &data.collisions {
            md.push_str(&format!(
                "| `{}` | `{}` | `{}` | {} |\n",
                c.key_value, c.key_type, c.location, c.message
            ));
        }
        md.push('\n');
    }

    if !data.unsafe_patterns.is_empty() {
        md.push_str("### 🟡 Unsafe Patterns (S006)\n\n");
        md.push_str("| Pattern | Line | Snippet |\n|---|---|---|\n");
        for p in &data.unsafe_patterns {
            let pat = format!("{:?}", p.pattern_type);
            md.push_str(&format!(
                "| {} | {} | `{}` |\n",
                pat,
                p.line,
                p.snippet.replace('|', "\\|")
            ));
        }
        md.push('\n');
    }

    if !data.event_issues.is_empty() {
        md.push_str("### 🔵 Event Inconsistencies (S008)\n\n");
        md.push_str("| Event | Issue Type | Location | Message |\n|---|---|---|---|\n");
        for e in &data.event_issues {
            let kind = format!("{:?}", e.issue_type);
            md.push_str(&format!(
                "| `{}` | {} | `{}` | {} |\n",
                e.event_name, kind, e.location, e.message
            ));
        }
        md.push('\n');
    }

    if !data.unhandled_results.is_empty() {
        md.push_str("### 🟠 Unhandled Results (S009)\n\n");
        md.push_str("> `Result` return values that are silently discarded can hide errors.\n\n");
        md.push_str("| Function | Call | Location | Message |\n|---|---|---|---|\n");
        for r in &data.unhandled_results {
            md.push_str(&format!(
                "| `{}` | `{}` | `{}` | {} |\n",
                r.function_name, r.call_expression, r.location, r.message
            ));
        }
        md.push('\n');
    }

    if !data.upgrade_findings.is_empty() {
        md.push_str("### 🟠 Upgrade / Admin Risks (S010)\n\n");
        md.push_str(
            "| Category | Function | Location | Message | Suggestion |\n|---|---|---|---|---|\n",
        );
        for f in &data.upgrade_findings {
            let cat = format!("{:?}", f.category);
            let func = f.function_name.as_deref().unwrap_or("—");
            md.push_str(&format!(
                "| {} | `{}` | `{}` | {} | {} |\n",
                cat, func, f.location, f.message, f.suggestion
            ));
        }
        md.push('\n');
    }

    if !data.smt_issues.is_empty() {
        md.push_str("### 🔴 SMT Invariant Violations (S011)\n\n");
        md.push_str("> Z3 formal verification found a provable invariant violation.\n\n");
        md.push_str("| Function | Location | Description |\n|---|---|---|\n");
        for s in &data.smt_issues {
            md.push_str(&format!(
                "| `{}` | `{}` | {} |\n",
                s.function_name, s.location, s.description
            ));
        }
        md.push('\n');
    }

    if !data.sep41_issues.is_empty() {
        md.push_str("### 🟡 SEP-41 Interface Deviations (S012)\n\n");
        md.push_str("| Function | Kind | Location | Message |\n|---|---|---|---|\n");
        for i in &data.sep41_issues {
            let kind = format!("{:?}", i.kind);
            md.push_str(&format!(
                "| `{}` | {} | `{}` | {} |\n",
                i.function_name, kind, i.location, i.message
            ));
        }
        md.push('\n');
    }

    if !data.vuln_matches.is_empty() {
        md.push_str("### 🛡️ Vulnerability Database Matches\n\n");
        md.push_str(
            "| ID | Name | Severity | File | Line | Description |\n|---|---|---|---|---|---|\n",
        );
        for m in &data.vuln_matches {
            md.push_str(&format!(
                "| `{}` | {} | {} | `{}` | {} | {} |\n",
                m.vuln_id,
                m.name,
                m.severity.to_uppercase(),
                m.file,
                m.line,
                m.description
            ));
        }
        md.push('\n');
    }

    if !data.timed_out_files.is_empty() {
        md.push_str("### ⏱️ Analysis Timeouts (S000)\n\n");
        md.push_str("> These files were not fully analysed due to the per-file timeout.\n\n");
        for f in &data.timed_out_files {
            md.push_str(&format!("- `{}`\n", f));
        }
        md.push('\n');
    }

    if total == 0 {
        md.push_str("_No findings — contract passed all checks._ 🎉\n\n");
    }

    // ── Footer ──
    md.push_str("---\n\n");
    md.push_str(&format!(
        "_Generated by [Sanctifier](https://github.com/HyperSafeD/Sanctifier) v{} on {}_\n",
        version, date
    ));

    md
}

// ── HTML renderer ─────────────────────────────────────────────────────────────

fn render_html(data: &ReportData, path: &Path, vuln_db_version: &str) -> String {
    // Embed the Markdown as sanitised HTML.  For a structured HTML doc we
    // convert the key sections manually to avoid pulling in a Markdown crate.
    let md = render_markdown(data, path, vuln_db_version);

    // Escape the raw Markdown so it can be embedded in a <pre> block, and
    // also generate a proper HTML document with a minimal stylesheet.
    let escaped_md = md
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;");

    let version = env!("CARGO_PKG_VERSION");
    let date = human_date();
    let overall_class = if data.has_critical {
        "critical"
    } else if data.has_high {
        "high"
    } else {
        "pass"
    };
    let overall_label = if data.has_critical {
        "CRITICAL"
    } else if data.has_high {
        "HIGH"
    } else {
        "PASS"
    };

    // Build summary rows
    let rows: &[(&str, &str, usize, &str)] = &[
        (
            "Authentication Gaps",
            "S001",
            data.auth_gaps.len(),
            "critical",
        ),
        (
            "Panic / Unwrap / Expect",
            "S002",
            data.panic_issues.len(),
            "high",
        ),
        (
            "Arithmetic Overflow",
            "S003",
            data.arithmetic_issues.len(),
            "high",
        ),
        (
            "Ledger Size Risk",
            "S004",
            data.size_warnings.len(),
            "medium",
        ),
        ("Storage Collisions", "S005", data.collisions.len(), "high"),
        (
            "Unsafe Patterns",
            "S006",
            data.unsafe_patterns.len(),
            "medium",
        ),
        (
            "Event Inconsistencies",
            "S008",
            data.event_issues.len(),
            "low",
        ),
        (
            "Unhandled Results",
            "S009",
            data.unhandled_results.len(),
            "high",
        ),
        ("Upgrade Risks", "S010", data.upgrade_findings.len(), "high"),
        (
            "SMT Invariant Violations",
            "S011",
            data.smt_issues.len(),
            "critical",
        ),
        (
            "SEP-41 Deviations",
            "S012",
            data.sep41_issues.len(),
            "medium",
        ),
        ("Vuln DB Matches", "—", data.vuln_matches.len(), "high"),
        (
            "Analysis Timeouts",
            "S000",
            data.timed_out_files.len(),
            "info",
        ),
    ];

    let total: usize = rows.iter().map(|r| r.2).sum();
    let mut summary_rows = String::new();
    for (cat, code, count, cls) in rows {
        let row_class = if *count > 0 { *cls } else { "zero" };
        summary_rows.push_str(&format!(
            "<tr class=\"{}\">\
               <td>{}</td><td><code>{}</code></td>\
               <td class=\"num\">{}</td>\
             </tr>\n",
            row_class, cat, code, count
        ));
    }
    summary_rows.push_str(&format!(
        "<tr class=\"total\">\
           <td><strong>Total</strong></td><td></td>\
           <td class=\"num\"><strong>{}</strong></td>\
         </tr>\n",
        total
    ));

    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>Sanctifier Security Report — {path}</title>
<style>
  :root {{
    --critical: #dc2626; --high: #ea580c; --medium: #d97706;
    --low: #2563eb; --pass: #16a34a; --info: #6b7280; --zero: #9ca3af;
  }}
  body {{ font-family: system-ui, sans-serif; max-width: 960px; margin: 2rem auto; padding: 0 1rem; color: #111; }}
  h1 {{ border-bottom: 2px solid #e5e7eb; padding-bottom: .5rem; }}
  h2 {{ margin-top: 2rem; border-bottom: 1px solid #e5e7eb; }}
  h3 {{ margin-top: 1.5rem; }}
  .badge {{ display:inline-block; padding:.25rem .75rem; border-radius:9999px; color:#fff; font-weight:700; font-size:.875rem; }}
  .badge.critical {{ background:var(--critical); }}
  .badge.high     {{ background:var(--high); }}
  .badge.pass     {{ background:var(--pass); }}
  table {{ border-collapse:collapse; width:100%; margin:.5rem 0 1.5rem; }}
  th,td {{ border:1px solid #e5e7eb; padding:.4rem .75rem; text-align:left; }}
  th {{ background:#f9fafb; }}
  td.num {{ text-align:right; font-variant-numeric:tabular-nums; }}
  tr.critical td {{ background:#fef2f2; }}
  tr.high td {{ background:#fff7ed; }}
  tr.medium td {{ background:#fffbeb; }}
  tr.low td {{ background:#eff6ff; }}
  tr.zero td {{ color:var(--zero); }}
  tr.total td {{ font-weight:700; background:#f9fafb; }}
  code {{ background:#f3f4f6; padding:.1rem .3rem; border-radius:.25rem; font-size:.875rem; }}
  pre {{ background:#f3f4f6; padding:1rem; border-radius:.5rem; overflow-x:auto; font-size:.8rem; }}
  footer {{ margin-top:3rem; color:var(--info); font-size:.8rem; border-top:1px solid #e5e7eb; padding-top:1rem; }}
</style>
</head>
<body>
<h1>Sanctifier Security Report</h1>
<table>
  <tr><th>Contract path</th><td><code>{path}</code></td></tr>
  <tr><th>Analysis date</th><td>{date}</td></tr>
  <tr><th>Tool version</th><td>{version}</td></tr>
  <tr><th>Vuln DB version</th><td>{vuln_db_version}</td></tr>
  <tr><th>Overall</th><td><span class="badge {overall_class}">{overall_label}</span></td></tr>
</table>

<h2>Summary</h2>
<table>
  <thead><tr><th>Category</th><th>Code</th><th>Count</th></tr></thead>
  <tbody>{summary_rows}</tbody>
</table>

<h2>Full Report (Markdown)</h2>
<pre>{escaped_md}</pre>

<footer>
  Generated by <a href="https://github.com/HyperSafeD/Sanctifier">Sanctifier</a> v{version} on {date}
</footer>
</body>
</html>
"#,
        path = path.display(),
        date = date,
        version = version,
        vuln_db_version = vuln_db_version,
        overall_class = overall_class,
        overall_label = overall_label,
        summary_rows = summary_rows,
        escaped_md = escaped_md,
    )
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn human_date() -> String {
    let now = std::time::SystemTime::now();
    let secs = now
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    // Simple ISO-8601 date from Unix timestamp (no external crate needed)
    let days_since_epoch = secs / 86400;
    // Compute year/month/day using the Gregorian calendar algorithm
    let z = days_since_epoch + 719468;
    let era = z / 146097;
    let doe = z % 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{:04}-{:02}-{:02}", y, m, d)
}
