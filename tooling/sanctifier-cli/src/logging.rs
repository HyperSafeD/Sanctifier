use tracing_subscriber::EnvFilter;

#[allow(dead_code)]
pub enum LogOutput {
    Text,
    Json,
}

/// Build an `EnvFilter` from a verbosity string without touching global state.
///
/// Accepts standard `tracing` directive syntax: `"warn"`, `"debug"`,
/// `"sanctifier=trace,warn"`, etc. Useful for measuring filter-build overhead
/// in benchmarks without side-effects.
pub fn make_filter(verbosity: &str) -> anyhow::Result<EnvFilter> {
    EnvFilter::try_new(verbosity)
        .map_err(|e| anyhow::anyhow!("invalid log filter '{}': {}", verbosity, e))
}

pub fn init(output: LogOutput) -> anyhow::Result<()> {
    let env_filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("warn"))
        .map_err(|err| anyhow::anyhow!("failed to configure log filter: {err}"))?;

    match output {
        LogOutput::Text => {
            tracing_subscriber::fmt()
                .with_env_filter(env_filter)
                .with_writer(std::io::stderr)
                .with_target(false)
                .without_time()
                .init();
        }
        LogOutput::Json => {
            tracing_subscriber::fmt()
                .json()
                .with_env_filter(env_filter)
                .with_writer(std::io::stderr)
                .with_current_span(false)
                .with_span_list(false)
                .init();
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    /// Budget for building a single `EnvFilter` — should be well under 50 ms
    /// even on a heavily loaded CI runner.
    const FILTER_BUILD_BUDGET_MS: u128 = 50;

    #[test]
    fn make_filter_accepts_standard_levels() {
        for level in &["error", "warn", "info", "debug", "trace"] {
            assert!(
                make_filter(level).is_ok(),
                "make_filter('{}') should succeed",
                level
            );
        }
    }

    #[test]
    fn make_filter_accepts_crate_qualified_directives() {
        assert!(make_filter("sanctifier=debug,warn").is_ok());
        assert!(make_filter("sanctifier_cli=trace").is_ok());
    }

    #[test]
    fn make_filter_builds_within_budget_for_all_modes() {
        let cases = [
            ("error", "quiet/error mode"),
            ("warn", "default mode"),
            ("info", "info mode"),
            ("debug", "verbose mode"),
            ("sanctifier=debug,warn", "crate-scoped verbose"),
        ];
        for (directive, label) in cases {
            let start = Instant::now();
            let _ = make_filter(directive).expect("filter must build");
            let elapsed = start.elapsed().as_millis();
            assert!(
                elapsed < FILTER_BUILD_BUDGET_MS,
                "make_filter for {label} took {elapsed}ms, budget {FILTER_BUILD_BUDGET_MS}ms"
            );
        }
    }

    #[test]
    fn make_filter_json_log_mode_builds_within_budget() {
        // JSON log mode uses the same EnvFilter; verify it meets the same budget.
        let start = Instant::now();
        let _ = make_filter("debug").expect("filter must build");
        let elapsed = start.elapsed().as_millis();
        assert!(
            elapsed < FILTER_BUILD_BUDGET_MS,
            "JSON-mode filter build took {elapsed}ms, budget {FILTER_BUILD_BUDGET_MS}ms"
        );
    }
}
