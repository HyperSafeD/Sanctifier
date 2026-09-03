#![allow(dead_code)]
use colored::Colorize;
use std::sync::atomic::{AtomicBool, Ordering};

static NO_COLOR: AtomicBool = AtomicBool::new(false);

/// Initialise the colour-engine from environment variables and (optional) CLI flags.
///
/// Priority (first wins):
/// 1. The caller can call `set_no_color(true)` to force monochrome.
/// 2. `NO_COLOR` environment variable (any non-empty value).
/// 3. `CLICOLOR_FORCE` / `CLICOLOR` — see `colored` crate docs.
/// 4. TTY auto-detection (handled by the `colored` crate).
pub fn init(cli_no_color: bool) {
    let no_color = cli_no_color
        || std::env::var("NO_COLOR").is_ok_and(|v| !v.is_empty())
        || std::env::var("CLICOLOR_FORCE")
            .is_err_and(|_| std::env::var("CLICOLOR").is_ok_and(|v| v == "0"));

    set_no_color(no_color);
}

/// Force-disable colour output regardless of TTY / env.
pub fn set_no_color(v: bool) {
    NO_COLOR.store(v, Ordering::Relaxed);
    if v {
        colored::control::set_override(true);
    } else {
        colored::control::unset_override();
    }
}

/// Returns `true` if colour output is currently suppressed.
pub fn is_no_color() -> bool {
    NO_COLOR.load(Ordering::Relaxed)
}

/// Convenience: colour a "✓" / "✅" green when colour is enabled, plain otherwise.
pub fn green_check() -> &'static str {
    if is_no_color() {
        "[OK]"
    } else {
        "✅"
    }
}

/// Convenience: colour a "✗" / "❌" red when colour is enabled, plain otherwise.
pub fn red_cross() -> &'static str {
    if is_no_color() {
        "[FAIL]"
    } else {
        "❌"
    }
}

/// Convenience: colour a "⚠️" yellow when colour is enabled, plain otherwise.
pub fn yellow_warning() -> &'static str {
    if is_no_color() {
        "[WARN]"
    } else {
        "⚠️"
    }
}

/// Convenience: colour a "ℹ️" / "🔍" blue when colour is enabled, plain otherwise.
pub fn blue_info() -> &'static str {
    if is_no_color() {
        "[INFO]"
    } else {
        "ℹ️"
    }
}

/// Wrap text in bold when colours are enabled.
pub fn bold(s: &str) -> String {
    if is_no_color() {
        s.to_string()
    } else {
        s.bold().to_string()
    }
}

/// Wrap text in red (for errors, arrows) when colours are enabled.
pub fn red(s: &str) -> String {
    if is_no_color() {
        s.to_string()
    } else {
        s.red().to_string()
    }
}

/// Wrap text in green (for success messages) when colours are enabled.
pub fn green(s: &str) -> String {
    if is_no_color() {
        s.to_string()
    } else {
        s.green().to_string()
    }
}

/// Wrap text in yellow (for warnings) when colours are enabled.
pub fn yellow(s: &str) -> String {
    if is_no_color() {
        s.to_string()
    } else {
        s.yellow().to_string()
    }
}

/// Wrap text in blue (for info) when colours are enabled.
pub fn blue(s: &str) -> String {
    if is_no_color() {
        s.to_string()
    } else {
        s.blue().to_string()
    }
}

/// Wrap text in cyan when colours are enabled.
pub fn cyan(s: &str) -> String {
    if is_no_color() {
        s.to_string()
    } else {
        s.cyan().to_string()
    }
}

/// Dim text when colours are enabled.
pub fn dimmed(s: &str) -> String {
    if is_no_color() {
        s.to_string()
    } else {
        s.dimmed().to_string()
    }
}

/// Combine green + bold.
pub fn green_bold(s: &str) -> String {
    if is_no_color() {
        s.to_string()
    } else {
        s.green().bold().to_string()
    }
}

/// Combine red + bold.
pub fn red_bold(s: &str) -> String {
    if is_no_color() {
        s.to_string()
    } else {
        s.red().bold().to_string()
    }
}

/// Combine blue + bold.
pub fn blue_bold(s: &str) -> String {
    if is_no_color() {
        s.to_string()
    } else {
        s.blue().bold().to_string()
    }
}

/// Combine yellow + bold.
pub fn yellow_bold(s: &str) -> String {
    if is_no_color() {
        s.to_string()
    } else {
        s.yellow().bold().to_string()
    }
}

/// Bright cyan for deploy/startup messages.
pub fn bright_cyan(s: &str) -> String {
    if is_no_color() {
        s.to_string()
    } else {
        s.bright_cyan().to_string()
    }
}

/// Bright yellow for build messages.
pub fn bright_yellow(s: &str) -> String {
    if is_no_color() {
        s.to_string()
    } else {
        s.bright_yellow().to_string()
    }
}
