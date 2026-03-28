use clap::Args;
use sanctifier_core::complexity::ContractMetrics;
use sanctifier_core::{analyze_complexity_from_source, render_text_report};
use std::fs;
use std::path::PathBuf;

#[derive(Args, Debug)]
pub struct ComplexityArgs {
    /// Path to a contract directory or Rust source file
    #[arg(default_value = ".")]
    pub path: PathBuf,
}

pub fn exec(args: ComplexityArgs) -> anyhow::Result<()> {
    let path = args.path;

    let source = if path.is_dir() {
        anyhow::bail!("Complexity command currently only supports file paths")
    } else {
        fs::read_to_string(&path)?
    };

    let metrics: ContractMetrics =
        match analyze_complexity_from_source(&source, path.to_string_lossy().as_ref()) {
            Ok(m) => m,
            Err(_) => ContractMetrics {
                contract_path: path.to_string_lossy().into_owned(),
                dependency_count: 0,
                functions: Vec::new(),
            },
        };

    let report = render_text_report(&metrics);
    println!("{}", report);
    Ok(())
}
