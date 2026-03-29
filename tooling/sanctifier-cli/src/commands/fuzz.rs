use anyhow::Context;
use clap::{Args, ValueEnum};
use sanctifier_core::fuzz::{FuzzHarnessGenerator, FuzzHarnessTarget};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum FuzzEngine {
    Afl,
    Honggfuzz,
    All,
}

#[derive(Args, Debug)]
pub struct FuzzArgs {
    /// Path to a contract directory, Cargo.toml, or a single Rust source file
    #[arg(default_value = ".")]
    pub path: PathBuf,

    /// Which harness backend to generate
    #[arg(long, value_enum, default_value_t = FuzzEngine::All)]
    pub engine: FuzzEngine,

    /// Directory where harness files will be written
    #[arg(short, long, default_value = "fuzz")]
    pub output_dir: PathBuf,

    /// Overwrite existing generated harness files
    #[arg(short, long)]
    pub force: bool,
}

pub fn exec(args: FuzzArgs) -> anyhow::Result<()> {
    let source_path = resolve_contract_source(&args.path)?;
    let source = fs::read_to_string(&source_path)
        .with_context(|| format!("failed to read source file: {}", source_path.display()))?;
    let spec = FuzzHarnessGenerator::inspect_source(&source, &source_path.display().to_string());

    fs::create_dir_all(&args.output_dir).with_context(|| {
        format!(
            "failed to create output directory: {}",
            args.output_dir.display()
        )
    })?;

    let targets = selected_targets(args.engine);
    for target in targets {
        let output_path = args.output_dir.join(target.file_name());
        if output_path.exists() && !args.force {
            anyhow::bail!(
                "output file already exists: {} (use --force to overwrite)",
                output_path.display()
            );
        }
        let rendered = FuzzHarnessGenerator::render(&spec, target);
        fs::write(&output_path, rendered)
            .with_context(|| format!("failed to write harness: {}", output_path.display()))?;
        println!(
            "Generated {} harness at {}",
            target.file_name(),
            output_path.display()
        );
    }

    Ok(())
}

fn selected_targets(engine: FuzzEngine) -> Vec<FuzzHarnessTarget> {
    match engine {
        FuzzEngine::Afl => vec![FuzzHarnessTarget::Afl],
        FuzzEngine::Honggfuzz => vec![FuzzHarnessTarget::Honggfuzz],
        FuzzEngine::All => vec![FuzzHarnessTarget::Afl, FuzzHarnessTarget::Honggfuzz],
    }
}

fn resolve_contract_source(path: &Path) -> anyhow::Result<PathBuf> {
    if path.is_file() {
        if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
            return Ok(path.to_path_buf());
        }
        if path.file_name().and_then(|name| name.to_str()) == Some("Cargo.toml") {
            let source = path
                .parent()
                .unwrap_or_else(|| Path::new("."))
                .join("src")
                .join("lib.rs");
            if source.exists() {
                return Ok(source);
            }
        }
    }

    let source = path.join("src").join("lib.rs");
    if source.exists() {
        Ok(source)
    } else {
        anyhow::bail!("could not resolve contract source from {}", path.display())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn resolves_directory_source() {
        let temp_dir = tempdir().unwrap();
        let src_dir = temp_dir.path().join("src");
        fs::create_dir_all(&src_dir).unwrap();
        fs::write(src_dir.join("lib.rs"), "pub fn demo() {}").unwrap();

        let resolved = resolve_contract_source(temp_dir.path()).unwrap();
        assert_eq!(resolved, src_dir.join("lib.rs"));
    }

    #[test]
    fn selects_all_targets() {
        let targets = selected_targets(FuzzEngine::All);
        assert_eq!(targets.len(), 2);
    }
}
