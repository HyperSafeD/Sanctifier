use clap::{Args, ValueEnum};
use sanctifier_core::gas_estimator::GasEstimator;
use sanctifier_core::gas_report::{
    detect_unbounded_loop_warnings, render_json_report, render_text_report, GasReport,
};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutputFormat {
    Text,
    Json,
}

#[derive(Args, Debug)]
pub struct GasArgs {
    /// Path to a Rust source file or contract directory
    #[arg(default_value = ".")]
    pub path: PathBuf,

    /// Output format
    #[arg(short, long, value_enum, default_value_t = OutputFormat::Text)]
    pub format: OutputFormat,
}

pub fn exec(args: GasArgs) -> anyhow::Result<()> {
    let sources = collect_sources(&args.path)?;
    let estimator = GasEstimator::new();

    let mut function_reports = Vec::new();
    let mut warnings = Vec::new();

    for source_path in sources {
        let source = fs::read_to_string(&source_path)?;
        function_reports.extend(estimator.estimate_contract(&source));
        warnings.extend(detect_unbounded_loop_warnings(&source));
    }

    let report = GasReport::from_estimates(function_reports);

    match args.format {
        OutputFormat::Json => println!("{}", render_json_report(&report)),
        OutputFormat::Text => println!("{}", render_text_report(&report, &warnings)),
    }

    Ok(())
}

fn collect_sources(path: &Path) -> anyhow::Result<Vec<PathBuf>> {
    if path.is_file() {
        return Ok(vec![path.to_path_buf()]);
    }

    if !path.is_dir() {
        anyhow::bail!("{} is not a valid Rust file or directory", path.display());
    }

    let mut files = Vec::new();
    collect_sources_inner(path, &mut files)?;
    files.sort();
    Ok(files)
}

fn collect_sources_inner(dir: &Path, out: &mut Vec<PathBuf>) -> anyhow::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            collect_sources_inner(&path, out)?;
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
            out.push(path);
        }
    }

    Ok(())
}
