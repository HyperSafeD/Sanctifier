use crate::commands::analyze::AnalyzeArgs;
use clap::Args;
use notify::{recommended_watcher, Event, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::time::Duration;
use tracing::warn;

#[derive(Args, Debug, Clone)]
pub struct WatchArgs {
    #[command(flatten)]
    pub analyze: AnalyzeArgs,

    /// Debounce file events before re-running analysis
    #[arg(long, default_value_t = 400)]
    pub debounce_ms: u64,

    /// Stop after N runs (useful for automation and tests)
    #[arg(long, hide = true)]
    pub max_runs: Option<usize>,
}

pub fn exec(args: WatchArgs) -> anyhow::Result<()> {
    let mut analyze_args = args.analyze.clone();
    if analyze_args.exit_code {
        eprintln!("Watch mode ignores --exit-code and keeps running until you stop it.");
        analyze_args.exit_code = false;
    }

    let watch_root = watch_root(&analyze_args.path);
    let (tx, rx) = mpsc::channel();
    let mut watcher = recommended_watcher(move |result| {
        let _ = tx.send(result);
    })?;

    watcher.watch(&watch_root, RecursiveMode::Recursive)?;
    if let Some(vuln_db_path) = &analyze_args.vuln_db {
        if vuln_db_path.exists() {
            watcher.watch(vuln_db_path, RecursiveMode::NonRecursive)?;
        }
    }

    println!("Watching {} for changes...", watch_root.display());

    let mut run_count = 0usize;
    run_analysis_cycle(&analyze_args, &mut run_count)?;
    if reached_max_runs(args.max_runs, run_count) {
        return Ok(());
    }

    let debounce = Duration::from_millis(args.debounce_ms);
    loop {
        match rx.recv() {
            Ok(Ok(event)) => {
                if !event_should_rerun(&event, analyze_args.vuln_db.as_deref()) {
                    continue;
                }
                drain_debounced_events(&rx, debounce, analyze_args.vuln_db.as_deref());
                println!("\nChange detected. Re-running analysis...\n");
                run_analysis_cycle(&analyze_args, &mut run_count)?;
                if reached_max_runs(args.max_runs, run_count) {
                    return Ok(());
                }
            }
            Ok(Err(error)) => {
                warn!(target: "sanctifier", error = %error, "File watcher error");
            }
            Err(_) => return Ok(()),
        }
    }
}

fn run_analysis_cycle(args: &AnalyzeArgs, run_count: &mut usize) -> anyhow::Result<()> {
    *run_count += 1;
    crate::commands::analyze::exec(args.clone())
}

fn reached_max_runs(max_runs: Option<usize>, run_count: usize) -> bool {
    max_runs.map(|max| run_count >= max).unwrap_or(false)
}

fn drain_debounced_events(
    rx: &mpsc::Receiver<notify::Result<Event>>,
    debounce: Duration,
    vuln_db_path: Option<&Path>,
) {
    loop {
        match rx.recv_timeout(debounce) {
            Ok(Ok(event)) => {
                if event_should_rerun(&event, vuln_db_path) {
                    continue;
                }
            }
            Ok(Err(error)) => {
                warn!(target: "sanctifier", error = %error, "File watcher error");
            }
            Err(RecvTimeoutError::Timeout) | Err(RecvTimeoutError::Disconnected) => break,
        }
    }
}

fn watch_root(path: &Path) -> PathBuf {
    if path.is_file() {
        path.parent()
            .map(|parent| parent.to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."))
    } else {
        path.to_path_buf()
    }
}

fn event_should_rerun(event: &Event, vuln_db_path: Option<&Path>) -> bool {
    event
        .paths
        .iter()
        .any(|path| is_relevant_path(path, vuln_db_path))
}

fn is_relevant_path(path: &Path, vuln_db_path: Option<&Path>) -> bool {
    if vuln_db_path
        .map(|candidate| candidate == path)
        .unwrap_or(false)
    {
        return true;
    }

    if path
        .file_name()
        .and_then(|name| name.to_str())
        .map(|name| name == "Cargo.toml" || name == ".sanctify.toml")
        .unwrap_or(false)
    {
        return true;
    }

    matches!(
        path.extension().and_then(|extension| extension.to_str()),
        Some("rs") | Some("toml")
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use notify::{Event, EventKind};

    #[test]
    fn relevant_rust_file_triggers_rerun() {
        let event = Event {
            kind: EventKind::Modify(notify::event::ModifyKind::Data(
                notify::event::DataChange::Content,
            )),
            paths: vec![PathBuf::from("src/lib.rs")],
            attrs: Default::default(),
        };

        assert!(event_should_rerun(&event, None));
    }

    #[test]
    fn unrelated_file_does_not_trigger_rerun() {
        let event = Event {
            kind: EventKind::Modify(notify::event::ModifyKind::Data(
                notify::event::DataChange::Content,
            )),
            paths: vec![PathBuf::from("README.md")],
            attrs: Default::default(),
        };

        assert!(!event_should_rerun(&event, None));
    }
}
