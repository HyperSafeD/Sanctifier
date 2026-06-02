use anyhow::{Context, Result};
use clap::Args;
use std::fs;
use std::path::PathBuf;
use toml_edit::{value, Array, DocumentMut, InlineTable, Item, Table, Value};

#[derive(Args)]
pub struct SuppressArgs {
    /// Finding code to suppress (e.g., S004)
    #[arg(required_unless_present = "list")]
    code: Option<String>,

    /// File path where the finding occurs
    #[arg(short, long)]
    file: Option<PathBuf>,

    /// Line number where the finding occurs
    #[arg(short, long)]
    line: Option<u32>,

    /// Reason for suppression
    #[arg(short, long)]
    reason: Option<String>,

    /// List all active suppressions
    #[arg(long)]
    list: bool,

    /// Path to .sanctify.toml (defaults to current directory)
    #[arg(long, default_value = ".sanctify.toml")]
    config: PathBuf,
}

pub fn exec(args: SuppressArgs) -> Result<()> {
    if args.list {
        list_suppressions(&args.config)?;
        return Ok(());
    }

    let code = args.code.context("Finding code is required")?;
    let file = args.file.context("--file is required")?;
    let line = args.line.context("--line is required")?;
    let reason = args.reason.unwrap_or_else(|| "Manual suppression".to_string());

    add_suppression(&args.config, &code, &file, line, &reason)?;

    println!("✅ Suppressed {} in {}:{}", code, file.display(), line);
    println!("   Reason: {}", reason);

    Ok(())
}

fn add_suppression(
    config_path: &PathBuf,
    code: &str,
    file: &PathBuf,
    line: u32,
    reason: &str,
) -> Result<()> {
    // Read or create config
    let content = if config_path.exists() {
        fs::read_to_string(config_path)
            .with_context(|| format!("Failed to read {}", config_path.display()))?
    } else {
        // Create minimal config
        String::from("[rules]\nenabled = true\n")
    };

    let mut doc = content
        .parse::<DocumentMut>()
        .context("Failed to parse .sanctify.toml")?;

    // Ensure [suppressions] table exists
    if !doc.contains_key("suppressions") {
        doc["suppressions"] = Item::Table(Table::new());
    }

    let suppressions = doc["suppressions"]
        .as_table_mut()
        .context("suppressions must be a table")?;

    // Get or create array for this code
    if !suppressions.contains_key(code) {
        suppressions[code] = value(Array::new());
    }

    let code_array = suppressions[code]
        .as_array_mut()
        .context("suppression entry must be an array")?;

    // Create suppression entry
    let mut entry = InlineTable::new();
    entry.insert("file", Value::from(file.display().to_string()));
    entry.insert("line", Value::from(line as i64));
    entry.insert("reason", Value::from(reason.to_owned()));

    code_array.push(toml_edit::Value::InlineTable(entry));

    // Write back
    fs::write(config_path, doc.to_string())
        .with_context(|| format!("Failed to write {}", config_path.display()))?;

    Ok(())
}

fn list_suppressions(config_path: &PathBuf) -> Result<()> {
    if !config_path.exists() {
        println!("No .sanctify.toml found. No suppressions configured.");
        return Ok(());
    }

    let content = fs::read_to_string(config_path)
        .with_context(|| format!("Failed to read {}", config_path.display()))?;

    let doc = content
        .parse::<DocumentMut>()
        .context("Failed to parse .sanctify.toml")?;

    let Some(suppressions) = doc.get("suppressions").and_then(|s| s.as_table()) else {
        println!("No suppressions configured.");
        return Ok(());
    };

    if suppressions.is_empty() {
        println!("No suppressions configured.");
        return Ok(());
    }

    println!("Active Suppressions:");
    println!();

    for (code, entries) in suppressions.iter() {
        let Some(array) = entries.as_array() else {
            continue;
        };

        for entry in array.iter() {
            let Some(table) = entry.as_inline_table() else {
                continue;
            };

            let file: &str = table
                .get("file")
                .and_then(|f: &toml_edit::Value| f.as_str())
                .unwrap_or("<unknown>");
            let line: i64 = table
                .get("line")
                .and_then(|l: &toml_edit::Value| l.as_integer())
                .unwrap_or(0);
            let reason: &str = table
                .get("reason")
                .and_then(|r: &toml_edit::Value| r.as_str())
                .unwrap_or("<no reason>");

            println!("  {} in {}:{}", code, file, line);
            println!("    Reason: {}", reason);
            println!();
        }
    }

    Ok(())
}
