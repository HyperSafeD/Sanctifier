use anyhow::{anyhow, Context};
use clap::{Args, ValueEnum};
use sanctifier_core::{Analyzer, SanctifyConfig, StorageCollisionIssue};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutputFormat {
    Text,
    Json,
}

#[derive(Args, Debug)]
pub struct StorageArgs {
    /// Path to a Rust source file, Cargo.toml, or contract directory
    #[arg(default_value = ".")]
    pub path: PathBuf,

    /// Output format
    #[arg(short, long, value_enum, default_value_t = OutputFormat::Text)]
    pub format: OutputFormat,
}

pub fn exec(args: StorageArgs) -> anyhow::Result<()> {
    let scan_root = normalize_scan_path(&args.path)?;
    let config = load_config(&scan_root)?;
    let analyzer = Analyzer::new(config.clone());

    let mut rust_files = collect_rust_files(&scan_root, &config.ignore_paths);
    rust_files.sort();

    if rust_files.is_empty() {
        return Err(anyhow!(
            "no Rust source files found under {}",
            scan_root.display()
        ));
    }

    let mut collisions = Vec::new();
    for file_path in rust_files {
        let source = fs::read_to_string(&file_path)
            .with_context(|| format!("failed to read {}", file_path.display()))?;
        let file_label = file_path.display().to_string();

        for mut collision in analyzer.scan_storage_collisions(&source) {
            collision.location = qualify_location(&file_label, &collision.location);
            collisions.push(collision);
        }
    }

    collisions.sort_by(|left, right| {
        left.location
            .cmp(&right.location)
            .then_with(|| left.key_value.cmp(&right.key_value))
            .then_with(|| left.key_type.cmp(&right.key_type))
    });

    match args.format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&collisions)?);
        }
        OutputFormat::Text => print_text_report(&collisions),
    }

    Ok(())
}

fn print_text_report(collisions: &[StorageCollisionIssue]) {
    if collisions.is_empty() {
        println!("No storage key collisions found.");
        return;
    }

    println!("Found {} storage key collision(s):", collisions.len());
    for collision in collisions {
        println!(
            "- {} [{}] at {}",
            collision.key_value, collision.key_type, collision.location
        );
        println!("  {}", collision.message);
    }
}

fn normalize_scan_path(path: &Path) -> anyhow::Result<PathBuf> {
    if path.is_dir() {
        return Ok(path.to_path_buf());
    }

    if path.file_name().and_then(|name| name.to_str()) == Some("Cargo.toml") {
        return path
            .parent()
            .map(Path::to_path_buf)
            .ok_or_else(|| anyhow!("{} has no parent directory", path.display()));
    }

    if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
        return Ok(path.to_path_buf());
    }

    Err(anyhow!(
        "expected a Rust source file, Cargo.toml, or directory, got {}",
        path.display()
    ))
}

fn qualify_location(file_label: &str, location: &str) -> String {
    if let Some((_, line)) = location.rsplit_once(':') {
        if line.parse::<usize>().is_ok() {
            return format!("{}:{}", file_label, line);
        }
    }

    format!("{}:{}", file_label, location)
}

fn collect_rust_files(path: &Path, ignore_paths: &[String]) -> Vec<PathBuf> {
    if path.is_file() {
        return vec![path.to_path_buf()];
    }

    let mut files = Vec::new();
    collect_rust_files_rec(path, ignore_paths, &mut files);
    files
}

fn collect_rust_files_rec(dir: &Path, ignore_paths: &[String], out: &mut Vec<PathBuf>) {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let name = path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("");

        if path.is_dir() {
            if ignore_paths.iter().any(|ignored| name.contains(ignored)) {
                continue;
            }
            collect_rust_files_rec(&path, ignore_paths, out);
            continue;
        }

        if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
            out.push(path);
        }
    }
}

fn load_config(path: &Path) -> anyhow::Result<SanctifyConfig> {
    let mut current = if path.is_file() {
        path.parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."))
    } else {
        path.to_path_buf()
    };

    loop {
        let config_path = current.join(".sanctify.toml");
        if config_path.exists() {
            let content = fs::read_to_string(&config_path)
                .with_context(|| format!("failed to read {}", config_path.display()))?;
            let config = toml::from_str(&content)
                .map_err(|error| anyhow!("failed to parse {}: {}", config_path.display(), error))?;
            return Ok(config);
        }

        if !current.pop() {
            break;
        }
    }

    Ok(SanctifyConfig::default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn qualify_location_prefers_file_and_line() {
        assert_eq!(
            qualify_location("contracts/sample.rs", "storage-op:42"),
            "contracts/sample.rs:42"
        );
    }

    #[test]
    fn normalize_scan_path_accepts_cargo_toml() {
        let path = PathBuf::from("/tmp/demo/Cargo.toml");
        assert_eq!(
            normalize_scan_path(&path).unwrap(),
            PathBuf::from("/tmp/demo")
        );
    }
}
