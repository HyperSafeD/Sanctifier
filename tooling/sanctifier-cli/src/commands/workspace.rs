#![allow(dead_code)]
use crate::commands::analyze::{analyze_single_file, collect_rs_files, run_with_timeout};
use crate::vulndb::VulnDatabase;
use clap::Args;
use colored::*;
use rayon::prelude::*;
use sanctifier_core::{Analyzer, SanctifyConfig};
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use std::time::Duration;
use toml;

// ── CLI args ──────────────────────────────────────────────────────────────────

#[derive(Args, Debug)]
pub struct WorkspaceArgs {
    /// Path to the workspace root (must contain a Cargo.toml with [workspace])
    #[arg(default_value = ".")]
    pub path: PathBuf,

    /// Per-file analysis timeout in seconds (0 = no timeout)
    #[arg(short, long, default_value = "30")]
    pub timeout: u64,

    /// Path to a custom vulnerability database JSON file
    #[arg(long)]
    pub vuln_db: Option<PathBuf>,

    /// Output format (text, json)
    #[arg(short, long, default_value = "text")]
    pub format: String,
}

// ── Minimal Cargo.toml deserialisation ───────────────────────────────────────

#[derive(Deserialize, Default)]
struct CargoManifest {
    #[serde(default)]
    workspace: Option<WorkspaceSection>,
    #[serde(default)]
    lib: Option<LibSection>,
}

#[derive(Deserialize, Default)]
struct WorkspaceSection {
    #[serde(default)]
    members: Vec<String>,
}

#[derive(Deserialize, Default)]
struct LibSection {
    #[serde(default, rename = "crate-type")]
    crate_type: Vec<String>,
}

// ── Member classification ─────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
enum MemberKind {
    /// Has `crate-type = ["cdylib"]` — a deployable Soroban contract.
    Contract,
    /// A regular Rust library shared across contracts.
    SharedLib,
    /// We could not determine the kind (treated as shared lib for analysis).
    Unknown,
}

struct WorkspaceMember {
    path: PathBuf,
    name: String,
    kind: MemberKind,
}

fn classify_member(member_dir: &Path) -> WorkspaceMember {
    let name = member_dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    let cargo_path = member_dir.join("Cargo.toml");
    let kind = match fs::read_to_string(&cargo_path)
        .ok()
        .and_then(|s| toml::from_str::<CargoManifest>(&s).ok())
    {
        Some(manifest) => {
            let is_cdylib = manifest
                .lib
                .as_ref()
                .map(|l| l.crate_type.iter().any(|t| t == "cdylib"))
                .unwrap_or(false);
            if is_cdylib {
                MemberKind::Contract
            } else {
                MemberKind::SharedLib
            }
        }
        None => MemberKind::Unknown,
    };

    WorkspaceMember {
        path: member_dir.to_path_buf(),
        name,
        kind,
    }
}

// ── Entry point ───────────────────────────────────────────────────────────────

pub fn exec(args: WorkspaceArgs) -> anyhow::Result<()> {
    let is_json = args.format == "json";
    let workspace_root = args.path.canonicalize().unwrap_or(args.path.clone());
    let workspace_cargo = workspace_root.join("Cargo.toml");

    if !workspace_cargo.exists() {
        anyhow::bail!(
            "No Cargo.toml found at {:?}. Pass the workspace root.",
            workspace_root
        );
    }

    let manifest_str = fs::read_to_string(&workspace_cargo)?;
    let manifest: CargoManifest = toml::from_str(&manifest_str)
        .map_err(|e| anyhow::anyhow!("Failed to parse {:?}: {}", workspace_cargo, e))?;

    let workspace_section = manifest.workspace.ok_or_else(|| {
        anyhow::anyhow!(
            "{:?} does not contain a [workspace] section.",
            workspace_cargo
        )
    })?;

    if workspace_section.members.is_empty() {
        println!("{} Workspace has no members.", "ℹ️".blue());
        return Ok(());
    }

    // Classify each member.
    let members: Vec<WorkspaceMember> = workspace_section
        .members
        .iter()
        .map(|m| classify_member(&workspace_root.join(m)))
        .collect();

    let contracts: Vec<&WorkspaceMember> = members
        .iter()
        .filter(|m| m.kind == MemberKind::Contract)
        .collect();
    let shared_libs: Vec<&WorkspaceMember> = members
        .iter()
        .filter(|m| m.kind != MemberKind::Contract)
        .collect();

    if !is_json {
        println!(
            "\n{} Workspace: {} contract(s), {} shared lib(s)",
            "🔍".cyan(),
            contracts.len(),
            shared_libs.len()
        );
        for lib in &shared_libs {
            println!("   {} Shared lib: {}", "📦".blue(), lib.name);
        }
        for c in &contracts {
            println!("   {} Contract:   {}", "📜".yellow(), c.name);
        }
        println!();
    }

    // Collect all .rs files from shared libraries (used as extra context).
    let ignore: Vec<String> = vec![];
    let shared_lib_files: Vec<PathBuf> = shared_libs
        .iter()
        .flat_map(|lib| collect_rs_files(&lib.path, &ignore))
        .collect();

    let vuln_db = Arc::new(match &args.vuln_db {
        Some(p) => VulnDatabase::load(p)?,
        None => VulnDatabase::load_default(),
    });

    let timeout_dur = if args.timeout == 0 {
        None
    } else {
        Some(Duration::from_secs(args.timeout))
    };

    let mut all_findings: Vec<(String, usize)> = Vec::new(); // (contract_name, finding_count)
    let mut grand_total = 0usize;

    for contract in &contracts {
        let config = load_config_for(&contract.path);
        let analyzer = Arc::new(Analyzer::new(config));

        // Collect contract source files + shared lib source files.
        let mut rs_files = collect_rs_files(&contract.path, &analyzer.config.ignore_paths);
        rs_files.extend(shared_lib_files.iter().cloned());
        rs_files.sort();
        rs_files.dedup();

        let total_files = rs_files.len();
        let counter = Arc::new(AtomicUsize::new(0));

        let results: Vec<_> = rs_files
            .par_iter()
            .map(|file_path| {
                let idx = counter.fetch_add(1, Ordering::Relaxed) + 1;
                if !is_json {
                    eprintln!("  [{}/{}] {}", idx, total_files, file_path.display());
                }
                let content = match fs::read_to_string(file_path) {
                    Ok(c) => c,
                    Err(_) => return Default::default(),
                };
                let file_name = file_path.display().to_string();
                let analyzer = Arc::clone(&analyzer);
                let vuln_db = Arc::clone(&vuln_db);
                let file_name_clone = file_name.clone();
                run_with_timeout(timeout_dur, move || {
                    analyze_single_file(&analyzer, &vuln_db, &content, &file_name_clone)
                })
                .unwrap_or_default()
            })
            .collect();

        let finding_count: usize = results.iter().map(count_findings).sum();
        grand_total += finding_count;
        all_findings.push((contract.name.clone(), finding_count));

        if !is_json {
            let icon = if finding_count == 0 {
                "✅".green()
            } else {
                "⚠️".yellow()
            };
            println!(
                "{} {} — {} finding(s)",
                icon,
                contract.name.bold(),
                finding_count
            );

            // Print per-category summaries.
            let auth: usize = results.iter().map(|r| r.auth_gaps.len()).sum();
            let arith: usize = results.iter().map(|r| r.arithmetic_issues.len()).sum();
            let panics: usize = results.iter().map(|r| r.panic_issues.len()).sum();
            let unhandled: usize = results.iter().map(|r| r.unhandled_results.len()).sum();
            let collisions: usize = results.iter().map(|r| r.collisions.len()).sum();

            if auth > 0 {
                println!("      auth gaps:        {}", auth);
            }
            if arith > 0 {
                println!("      arithmetic:       {}", arith);
            }
            if panics > 0 {
                println!("      panic/unwrap:     {}", panics);
            }
            if unhandled > 0 {
                println!("      unhandled result: {}", unhandled);
            }
            if collisions > 0 {
                println!("      key collisions:   {}", collisions);
            }
        }
    }

    if is_json {
        let report = serde_json::json!({
            "workspace": workspace_root.display().to_string(),
            "contracts": all_findings.iter().map(|(name, count)| {
                serde_json::json!({ "name": name, "total_findings": count })
            }).collect::<Vec<_>>(),
            "shared_libs": shared_libs.iter().map(|l| &l.name).collect::<Vec<_>>(),
            "grand_total_findings": grand_total,
        });
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        println!(
            "\n{} Grand total: {} finding(s) across {} contract(s)",
            "📊".cyan(),
            grand_total,
            contracts.len()
        );
    }

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
        + r.upgrade_reports
            .iter()
            .map(|u| u.findings.len())
            .sum::<usize>()
        + r.smt_issues.len()
        + r.sep41_issues.len()
        + r.truncation_bounds_issues.len()
        + r.vuln_matches.len()
        + r.timed_out as usize
}

fn load_config_for(path: &Path) -> SanctifyConfig {
    let mut current = path.to_path_buf();
    loop {
        let config_path = current.join(".sanctify.toml");
        if config_path.exists() {
            if let Ok(content) = fs::read_to_string(&config_path) {
                if let Ok(config) = toml::from_str(&content) {
                    return config;
                }
            }
        }
        if !current.pop() {
            break;
        }
    }
    SanctifyConfig::default()
}
