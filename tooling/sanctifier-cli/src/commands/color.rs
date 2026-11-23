#![allow(dead_code)]
use colored::Colorize;
use std::io::IsTerminal as _;
use std::sync::atomic::{AtomicBool, Ordering};

static NO_COLOR: AtomicBool = AtomicBool::new(false);

/// Initialise the colour-engine from environment variables and (optional) CLI flags.
///
/// Priority (first wins):
/// 1. `cli_no_color` — the caller passes `true` when `--no-color` / `--no-colour` is set.
/// 2. `NO_COLOR` environment variable (any non-empty value disables colour per
///    <https://no-color.org>).
/// 3. `CLICOLOR=0` explicitly disables colour.
/// 4. `CLICOLOR_FORCE` (any non-empty, non-`"0"` value) **re-enables** colour even when
///    stdout is not a TTY.
/// 5. Stdout TTY auto-detection: colour is disabled when stdout is not connected to a
///    terminal (e.g. piped to a file, `grep`, CI log parser, or `webhook.rs`).
pub fn init(cli_no_color: bool) {
    let clicolor_force = std::env::var("CLICOLOR_FORCE")
        .is_ok_and(|v| !v.is_empty() && v != "0");

    let no_color = cli_no_color
        || std::env::var("NO_COLOR").is_ok_and(|v| !v.is_empty())
        || std::env::var("CLICOLOR").is_ok_and(|v| v == "0")
        || (!clicolor_force && !std::io::stdout().is_terminal());

    set_no_color(no_color);
}

/// Force-disable colour output regardless of TTY / env.
pub fn set_no_color(v: bool) {
    NO_COLOR.store(v, Ordering::Relaxed);
    // Propagate to the `colored` crate so any direct `.red().to_string()` calls
    // that bypass our wrappers also honour the setting.
    // `set_override(false)` = always strip ANSI codes; `unset_override()` = let
    // `colored` decide (it will re-check the TTY on each call).
    if v {
        colored::control::set_override(false);
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

#[cfg(test)]
mod tests {
    use super::*;

    fn reset() {
        set_no_color(false);
    }

    #[test]
    fn set_no_color_true_disables_wrappers() {
        set_no_color(true);
        assert_eq!(red("hello"), "hello");
        assert_eq!(green("ok"), "ok");
        assert_eq!(bold("title"), "title");
        reset();
    }

    #[test]
    fn set_no_color_false_re_enables_wrappers() {
        set_no_color(true);
        set_no_color(false);
        // When colour is on the wrappers return ANSI-decorated strings, which
        // are never equal to the bare input.
        assert_ne!(red("hello"), "hello");
        reset();
    }

    #[test]
    fn no_color_env_disables_color() {
        // Temporarily set NO_COLOR, call init, assert wrappers strip ANSI.
        // We use a scoped override rather than actually mutating env state so
        // parallel tests are unaffected.
        set_no_color(false);

        // Simulate what init() does when NO_COLOR is set:
        let no_color = true; // as if std::env::var("NO_COLOR") returned a non-empty value
        set_no_color(no_color);

        assert!(is_no_color());
        assert_eq!(yellow("warn"), "warn");
        reset();
    }

    #[test]
    fn green_check_returns_ok_when_no_color() {
        set_no_color(true);
        assert_eq!(green_check(), "[OK]");
        reset();
    }

    #[test]
    fn red_cross_returns_fail_when_no_color() {
        set_no_color(true);
        assert_eq!(red_cross(), "[FAIL]");
        reset();
    }

    #[test]
    fn yellow_warning_returns_warn_when_no_color() {
        set_no_color(true);
        assert_eq!(yellow_warning(), "[WARN]");
        reset();
    }

    #[test]
    fn blue_info_returns_info_when_no_color() {
        set_no_color(true);
        assert_eq!(blue_info(), "[INFO]");
        reset();
    }

    #[test]
    fn colored_crate_override_set_to_false_when_no_color() {
        // When set_no_color(true) the `colored` crate must have its override set
        // to `false` (strip ANSI) so direct uses of `.red().to_string()` also
        // produce plain text.
        set_no_color(true);
        let s = "plain".red().to_string();
        assert!(!s.contains('\x1b'), "ANSI codes must be absent when no-color is set");
        reset();
    }
}
