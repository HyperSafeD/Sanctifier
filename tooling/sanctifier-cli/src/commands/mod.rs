pub mod analyze;
pub mod badge;
pub mod benchmark;
pub mod callgraph;
pub mod color;
pub mod complexity;
pub mod deploy;
pub mod diff;
pub mod doctor;
pub mod explain;
pub mod export;
pub mod fix;
pub mod gas;
pub mod harness;
pub mod init;
pub mod install_hooks;
pub mod lsp;
pub mod pr_comment;
pub mod reentrancy;
pub mod report;
pub mod report_templates;
pub mod sarif;
pub mod serve;
pub mod storage;
pub mod suppress;
pub mod update;
pub mod upgrade;
pub mod verify;
pub mod verify_deployment;
pub mod watch;
pub mod webhook;
pub mod workspace;

/// Shared test-only synchronization for `std::env::set_current_dir`, which is
/// process-wide. Any test in this crate that changes the current directory
/// must hold this lock for its entire duration (including restoring the
/// original directory), or concurrently-running tests can race: one test may
/// capture another test's temp directory as its "original" CWD just before
/// that temp directory is dropped and deleted, causing `set_current_dir` to
/// fail with `NotFound` and poisoning any lock held at the time.
#[cfg(test)]
pub(crate) mod test_support {
    use std::sync::Mutex;

    pub(crate) static CWD_LOCK: Mutex<()> = Mutex::new(());
}
