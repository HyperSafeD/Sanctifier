//! `sanctifier reentrancy` — run the reentrancy detector on a contract file.

use clap::Args;
use colored::*;
use sanctifier_core::rules::reentrancy::ReentrancyRule;
use sanctifier_core::rules::{Rule, Severity};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Args, Debug)]
pub struct ReentrancyArgs {
    /// Path to a Rust source file or contract directory
    #[arg(default_value = ".")]
    pub path: PathBuf,

    /// Output format: text or json
    #[arg(short, long, default_value = "text")]
    pub format: String,

    /// Also emit auto-fix patches (text mode only)
    #[arg(long)]
    pub fix: bool,
}

pub fn exec(args: ReentrancyArgs) -> anyhow::Result<()> {
    let path = &args.path;

    let source = if path.is_file() {
        fs::read_to_string(path)?
    } else if path.is_dir() {
        let mut combined = String::new();
        collect_rs_source(path, &mut combined);
        combined
    } else {
        anyhow::bail!("Path does not exist: {}", path.display());
    };

    let rule = ReentrancyRule::new();
    let violations = rule.check(&source);

    if args.format == "json" {
        println!("{}", serde_json::to_string_pretty(&violations)?);
        return Ok(());
    }

    // ── Text output ──────────────────────────────────────────────────────────
    if violations.is_empty() {
        println!("{} No reentrancy vulnerabilities found.", "✅".green());
    } else {
        println!(
            "\n{} Found {} reentrancy vulnerability(ies)!",
            "❌".red(),
            violations.len()
        );
        for v in &violations {
            let sev_icon = match v.severity {
                Severity::Error => "❌".red(),
                Severity::Warning => "⚠️".yellow(),
                Severity::Info | _ => "ℹ️".blue(),
            };
            println!("   {} [S013] {}", sev_icon, v.message.bold());
            println!("      Location: {}", v.location);
            if let Some(suggestion) = &v.suggestion {
                println!("      Suggestion: {}", suggestion);
            }
        }
    }

    if args.fix {
        let patches = rule.fix(&source);
        if patches.is_empty() {
            println!("\n{} No auto-fix patches available.", "ℹ️".blue());
        } else {
            println!("\n{} Auto-fix patches ({}):", "🔧".cyan(), patches.len());
            for patch in &patches {
                println!("   • {} (line {})", patch.description, patch.start_line);
            }
        }
    }

    Ok(())
}

fn collect_rs_source(dir: &Path, out: &mut String) {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let p = entry.path();
        if p.is_dir() {
            let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if name == "target" || name == ".git" {
                continue;
            }
            collect_rs_source(&p, out);
        } else if p.extension().and_then(|s| s.to_str()) == Some("rs") {
            if let Ok(content) = fs::read_to_string(&p) {
                out.push_str(&content);
                out.push('\n');
            }
        }
    }
}
