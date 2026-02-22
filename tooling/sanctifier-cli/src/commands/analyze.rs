use std::fs;
use std::path::{Path, PathBuf};
use clap::Args;
use colored::*;

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
    
    println!("{} Scaffolding: Analyze command placeholder success message.", "✅".green());
    
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
