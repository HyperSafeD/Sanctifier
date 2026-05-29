use crate::commands::analyze::{
    analyze_single_file, collect_rs_files, load_config, run_with_timeout,
};
use crate::vulndb::VulnDatabase;
use clap::{Args, ValueEnum};
use colored::*;
use rayon::prelude::*;
use sanctifier_core::Analyzer;
use std::fs;
use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use std::time::Duration;

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExportFormat {
    /// Comma-separated values (opens directly in Excel / Google Sheets).
    Csv,
    /// Tab-separated values with a .tsv extension.
    Tsv,
}

impl ExportFormat {
    fn delimiter(&self) -> u8 {
        match self {
            ExportFormat::Csv => b',',
            ExportFormat::Tsv => b'\t',
        }
    }

    fn default_extension(&self) -> &'static str {
        match self {
            ExportFormat::Csv => "csv",
            ExportFormat::Tsv => "tsv",
        }
    }
}

#[derive(Args, Debug)]
pub struct ExportArgs {
    /// Path to the contract directory, workspace, or a single .rs file
    #[arg(default_value = ".")]
    pub path: PathBuf,

    /// Export format
    #[arg(short, long, value_enum, default_value_t = ExportFormat::Csv)]
    pub format: ExportFormat,

    /// Output file path (defaults to `sanctifier-findings.<ext>` in the current directory)
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// Path to a custom vulnerability database JSON file
    #[arg(long)]
    pub vuln_db: Option<PathBuf>,

    /// Per-file analysis timeout in seconds (0 = no timeout)
    #[arg(short, long, default_value = "30")]
    pub timeout: u64,
}

pub fn exec(args: ExportArgs) -> anyhow::Result<()> {
    let output_path = args.output.clone().unwrap_or_else(|| {
        PathBuf::from(format!(
            "sanctifier-findings.{}",
            args.format.default_extension()
        ))
    });

    let config = load_config(&args.path);
    let analyzer = Arc::new(Analyzer::new(config));

    let vuln_db = Arc::new(match &args.vuln_db {
        Some(p) => VulnDatabase::load(p)?,
        None => VulnDatabase::load_default(),
    });

    let rs_files = if args.path.is_dir() {
        collect_rs_files(&args.path, &analyzer.config.ignore_paths)
    } else if args.path.extension().and_then(|s| s.to_str()) == Some("rs") {
        vec![args.path.clone()]
    } else {
        vec![]
    };

    if rs_files.is_empty() {
        eprintln!("{} No Rust source files found at {:?}", "⚠️".yellow(), args.path);
        return Ok(());
    }

    let total = rs_files.len();
    let counter = Arc::new(AtomicUsize::new(0));
    let timeout_dur = if args.timeout == 0 {
        None
    } else {
        Some(Duration::from_secs(args.timeout))
    };

    let results: Vec<_> = rs_files
        .par_iter()
        .map(|file_path| {
            let idx = counter.fetch_add(1, Ordering::Relaxed) + 1;
            eprintln!("[{}/{}] Analyzing {}", idx, total, file_path.display());
            let content = match fs::read_to_string(file_path) {
                Ok(c) => c,
                Err(_) => return Default::default(),
            };
            let file_name = file_path.display().to_string();
            let analyzer = Arc::clone(&analyzer);
            let vuln_db = Arc::clone(&vuln_db);
            let file_name_clone = file_name.clone();
            match run_with_timeout(timeout_dur, move || {
                analyze_single_file(&analyzer, &vuln_db, &content, &file_name_clone)
            }) {
                Some(r) => r,
                None => Default::default(),
            }
        })
        .collect();

    // ── Flatten all findings into CSV rows ────────────────────────────────────

    let file = fs::File::create(&output_path)?;
    let mut wtr = csv::WriterBuilder::new()
        .delimiter(args.format.delimiter())
        .from_writer(file);

    wtr.write_record(["severity", "code", "category", "title", "location", "suggestion"])?;

    for r in &results {
        for item in &r.auth_gaps {
            wtr.write_record([
                "critical",
                "S001",
                "auth_gap",
                "Missing require_auth()",
                &item.function_name,
                "Add <addr>.require_auth() at the start of the function",
            ])?;
        }
        for item in &r.panic_issues {
            wtr.write_record([
                "high",
                "S002",
                "panic_handling",
                &format!("Explicit {} usage", item.issue_type),
                &item.location,
                "Replace with Result-returning error handling",
            ])?;
        }
        for item in &r.arithmetic_issues {
            wtr.write_record([
                "high",
                "S003",
                "arithmetic",
                &format!("Unchecked {} operation", item.operation),
                &item.location,
                "Use checked_add / checked_mul or enable overflow-checks = true",
            ])?;
        }
        for item in &r.size_warnings {
            wtr.write_record([
                "medium",
                "S004",
                "storage_limits",
                &format!("Ledger size risk: {}", item.struct_name),
                &item.struct_name,
                "Reduce struct size or split storage across multiple entries",
            ])?;
        }
        for item in &r.collisions {
            wtr.write_record([
                "medium",
                "S005",
                "storage_keys",
                "Storage key collision",
                &item.location,
                &item.message,
            ])?;
        }
        for item in &r.unsafe_patterns {
            wtr.write_record([
                "high",
                "S006",
                "unsafe_patterns",
                "Unsafe pattern detected",
                &item.snippet,
                "Avoid unsafe Rust patterns in Soroban contracts",
            ])?;
        }
        for item in &r.custom_matches {
            wtr.write_record([
                "medium",
                "S007",
                "custom_rules",
                &item.rule_name,
                &item.snippet,
                "",
            ])?;
        }
        for item in &r.event_issues {
            wtr.write_record([
                "low",
                "S008",
                "event_consistency",
                "Event consistency issue",
                &item.location,
                &item.message,
            ])?;
        }
        for item in &r.unhandled_results {
            wtr.write_record([
                "high",
                "S009",
                "error_handling",
                "Unhandled Result value",
                &item.location,
                "Propagate or explicitly handle the Result",
            ])?;
        }
        for report in &r.upgrade_reports {
            for finding in &report.findings {
                wtr.write_record([
                    "high",
                    "S010",
                    "upgrade_safety",
                    &finding.message,
                    &finding.location,
                    "",
                ])?;
            }
        }
        for item in &r.smt_issues {
            wtr.write_record([
                "critical",
                "S011",
                "smt_verification",
                "SMT invariant violation",
                &item.location,
                &item.suggestion,
            ])?;
        }
        for item in &r.sep41_issues {
            wtr.write_record([
                "medium",
                "S012",
                "token_interface",
                "SEP-41 interface deviation",
                &item.location,
                &item.message,
            ])?;
        }
        for item in &r.truncation_bounds_issues {
            wtr.write_record([
                "high",
                "S016",
                "bounds_checking",
                &format!("Truncation/bounds risk: {}", item.kind),
                &item.location,
                &item.suggestion,
            ])?;
        }
        for item in &r.vuln_matches {
            wtr.write_record([
                &item.severity,
                "S014",
                "vuln_db",
                &item.name,
                &item.location,
                &item.description,
            ])?;
        }
        if r.timed_out {
            wtr.write_record([
                "low",
                "S000",
                "analysis_timeout",
                "Analysis timed out",
                &r.file_path,
                "Increase --timeout or simplify the contract",
            ])?;
        }
    }

    wtr.flush()?;

    let total_rows = results.iter().map(count_findings).sum::<usize>();
    println!(
        "{} Exported {} finding(s) to {:?}",
        "✅".green(),
        total_rows,
        output_path
    );
    Ok(())
}

fn count_findings(r: &crate::commands::analyze::FileAnalysisResult) -> usize {
    r.auth_gaps.len()
        + r.panic_issues.len()
        + r.arithmetic_issues.len()
        + r.size_warnings.len()
        + r.collisions.len()
        + r.unsafe_patterns.len()
        + r.custom_matches.len()
        + r.event_issues.len()
        + r.unhandled_results.len()
        + r.upgrade_reports.iter().map(|u| u.findings.len()).sum::<usize>()
        + r.smt_issues.len()
        + r.sep41_issues.len()
        + r.truncation_bounds_issues.len()
        + r.vuln_matches.len()
        + r.timed_out as usize
}
