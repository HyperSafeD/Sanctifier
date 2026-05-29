use crate::commands::analyze::{run_analysis, AnalyzeArgs};
use notify_debouncer_mini::{new_debouncer, notify::*};
use std::sync::mpsc::channel;
use std::time::Duration;

#[derive(clap::Args, Debug, Clone)]
pub struct WatchArgs {
    #[command(flatten)]
    pub analyze_args: AnalyzeArgs,

    /// Disable clearing the terminal screen before each re-analysis
    #[arg(long)]
    pub no_clear: bool,
}

pub fn exec(args: WatchArgs) -> anyhow::Result<()> {
    let path = args.analyze_args.path.clone();

    // Ensure the path exists
    if !path.exists() {
        anyhow::bail!("Path {:?} does not exist", path);
    }

    println!("🚀 Starting Sanctifier in watch mode...");

    // Initial analysis
    let _ = run_analysis(args.analyze_args.clone());

    let (tx, rx) = channel();

    // Setup debouncer with 500ms delay to avoid multiple triggers for a single save
    let mut debouncer = new_debouncer(Duration::from_millis(500), tx)?;

    // Watch the directory recursively
    debouncer.watcher().watch(&path, RecursiveMode::Recursive)?;

    println!("\n👀 Watching for changes in {:?}...", path);
    println!("Press Ctrl+C to stop.");

    for res in rx {
        match res {
            Ok(events) => {
                let mut should_rerun = false;
                for event in events {
                    if is_relevant_file(&event.path) {
                        should_rerun = true;
                        break;
                    }
                }

                if should_rerun {
                    if !args.no_clear {
                        // ANSI escape sequence to clear screen and reset cursor position
                        // \x1B[2J clears the screen, \x1B[1;1H moves cursor to top-left
                        print!("\x1B[2J\x1B[1;1H");
                    }
                    println!("\n🔄 Change detected, re-running analysis...");
                    let _ = run_analysis(args.analyze_args.clone());
                    println!("\n👀 Watching for changes in {:?}...", path);
                }
            }
            Err(error) => {
                eprintln!("Watch error: {:?}", error);
            }
        }
    }

    Ok(())
}

/// Determines if a file change should trigger a re-analysis.
fn is_relevant_file(path: &std::path::Path) -> bool {
    // We care about Rust files
    if let Some(ext) = path.extension() {
        if ext == "rs" {
            return true;
        }
    }
    // And configuration files
    if let Some(name) = path.file_name() {
        if name == ".sanctify.toml" || name == "Cargo.toml" {
            return true;
        }
    }
    false
}
