use std::fs;
use std::path::{Path, PathBuf};
use clap::Args;
use colored::*;
use sanctifier_core::{Analyzer, SanctifyConfig};

#[derive(Args, Debug)]
pub struct AnalyzeArgs {
    /// Path to the contract directory or Cargo.toml
    #[arg(default_value = ".")]
    pub path: PathBuf,
}

pub fn exec(args: AnalyzeArgs) -> anyhow::Result<()> {
    let path = &args.path;

    if !is_soroban_project(path) {
        eprintln!(
            "{} Error: {:?} is not a valid Soroban project. (Missing Cargo.toml with 'soroban-sdk' dependency)",
            "❌".red(),
            path
        );
        std::process::exit(1);
    }

    println!(
        "{} Sanctifier: Valid Soroban project found at {:?}",
        "✨".green(),
        path
    );
    
    let config = SanctifyConfig::default();
    let analyzer = Analyzer::new(config);
    
    let mut collisions = Vec::new();

    if path.is_dir() {
        walk_dir(path, &analyzer, &mut collisions)?;
    } else {
        if path.extension().and_then(|s| s.to_str()) == Some("rs") {
            if let Ok(content) = fs::read_to_string(path) {
                collisions.extend(analyzer.scan_storage_collisions(&content));
            }
        }
    }

    if collisions.is_empty() {
        println!("\n{} No storage key collisions found.", "✅".green());
    } else {
        println!("\n{} Found potential Storage Key Collisions!", "⚠️".yellow());
        for collision in collisions {
            println!("   {} Value: {}", "->".red(), collision.key_value.bold());
            println!("      Type: {}", collision.key_type);
            println!("      Location: {}", collision.location);
            println!("      Message: {}", collision.message);
        }
    }
    
    Ok(())
}

fn walk_dir(dir: &Path, analyzer: &Analyzer, collisions: &mut Vec<sanctifier_core::StorageCollisionIssue>) -> anyhow::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            walk_dir(&path, analyzer, collisions)?;
        } else if path.extension().and_then(|s| s.to_str()) == Some("rs") {
            if let Ok(content) = fs::read_to_string(&path) {
                let mut issues = analyzer.scan_storage_collisions(&content);
                // Prefix location with filename
                let file_name = path.display().to_string();
                for issue in &mut issues {
                    issue.location = format!("{}:{}", file_name, issue.location);
                }
                collisions.extend(issues);
            }
        }
    }
    Ok(())
}

fn is_soroban_project(path: &Path) -> bool {
    let cargo_toml_path = if path.is_dir() {
        path.join("Cargo.toml")
    } else if path.file_name().and_then(|s| s.to_str()) == Some("Cargo.toml") {
        path.to_path_buf()
    } else {
        // If it's a file but not Cargo.toml, try looking in parents
        let mut current = path.parent();
        while let Some(p) = current {
            let cargo = p.join("Cargo.toml");
            if cargo.exists() {
                if let Ok(content) = fs::read_to_string(cargo) {
                    if content.contains("soroban-sdk") {
                        return true;
                    }
                }
            }
            current = p.parent();
        }
        return false;
    };

    if !cargo_toml_path.exists() {
        return false;
    }

    if let Ok(content) = fs::read_to_string(cargo_toml_path) {
        content.contains("soroban-sdk")
    } else {
        false
    }
}
