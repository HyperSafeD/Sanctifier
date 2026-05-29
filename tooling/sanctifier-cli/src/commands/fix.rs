use clap::Args;
use colored::*;
use sanctifier_core::{patcher::Patcher, rules::Patch, RuleRegistry};
use std::fs;
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};

#[derive(Args, Debug)]
pub struct FixArgs {
    /// Path to a contract file or directory
    #[arg(default_value = ".")]
    pub path: PathBuf,
    /// Interactively review each patch before applying
    #[arg(long)]
    pub interactive: bool,
}

pub fn exec(args: FixArgs) -> anyhow::Result<()> {
    let registry = RuleRegistry::with_default_rules();
    let files = collect_rs_files(&args.path);

    let mut total_applied = 0usize;
    let mut total_skipped = 0usize;
    let mut apply_all = false;

    for file_path in &files {
        let source = match fs::read_to_string(file_path) {
            Ok(s) => s,
            Err(_) => continue,
        };

        let violations = registry.run_all(&source);
        let all_patches: Vec<Patch> = violations
            .iter()
            .flat_map(|v| v.patches.iter().cloned())
            .collect();

        if all_patches.is_empty() {
            continue;
        }

        println!("\n{} {}", "📄".blue(), file_path.display());

        let selected: Vec<Patch> = if !args.interactive || apply_all {
            all_patches.clone()
        } else {
            let mut chosen = Vec::new();
            for patch in &all_patches {
                if apply_all {
                    chosen.push(patch.clone());
                    continue;
                }

                println!("\n  {} {}", "→".yellow(), patch.description);
                print_diff(&source, patch);

                loop {
                    print!("  Apply? [y/n/a/d/?] ");
                    io::stdout().flush()?;
                    let mut line = String::new();
                    io::stdin().lock().read_line(&mut line)?;
                    match line.trim() {
                        "y" | "Y" => {
                            chosen.push(patch.clone());
                            break;
                        }
                        "n" | "N" => {
                            total_skipped += 1;
                            break;
                        }
                        "a" | "A" => {
                            apply_all = true;
                            chosen.push(patch.clone());
                            break;
                        }
                        "d" | "D" => {
                            print_diff(&source, patch);
                        }
                        _ => {
                            println!(
                                "  y=apply  n=skip  a=apply-all-remaining  d=show-diff  ?=help"
                            );
                        }
                    }
                }
            }
            chosen
        };

        if selected.is_empty() {
            continue;
        }

        let patched = Patcher::apply_patches(&source, &selected);
        fs::write(file_path, patched)?;
        total_applied += selected.len();
        println!("  {} Applied {} patch(es)", "✅".green(), selected.len());
    }

    println!(
        "\n{} Done: {} applied, {} skipped",
        "✨".green(),
        total_applied,
        total_skipped
    );
    Ok(())
}

fn print_diff(source: &str, patch: &Patch) {
    let lines: Vec<&str> = source.lines().collect();
    let start = patch.start_line.saturating_sub(1);
    let end = patch.end_line.min(lines.len());
    println!("  {}", "──────".dimmed());
    for (i, line) in lines[start..end].iter().enumerate() {
        println!("  {:<6} {}", format!("-{}", start + i + 1).red(), line);
    }
    for new_line in patch.replacement.lines() {
        println!("  {:<6} {}", "+".green(), new_line);
    }
    println!("  {}", "──────".dimmed());
}

fn collect_rs_files(path: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if path.is_file() {
        if path.extension().and_then(|s| s.to_str()) == Some("rs") {
            out.push(path.to_path_buf());
        }
    } else if path.is_dir() {
        walk_dir(path, &mut out);
    }
    out
}

fn walk_dir(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk_dir(&path, out);
        } else if path.extension().and_then(|s| s.to_str()) == Some("rs") {
            out.push(path);
        }
    }
}
