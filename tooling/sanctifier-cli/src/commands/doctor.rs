use clap::Args;
use colored::Colorize;
use std::process::Command;

#[derive(Args, Debug)]
pub struct DoctorArgs {
    /// Show verbose output for each check
    #[arg(short, long)]
    pub verbose: bool,
}

struct CheckResult {
    name: &'static str,
    passed: bool,
    detail: String,
}

pub fn exec(args: DoctorArgs) -> anyhow::Result<()> {
    println!("{}", "sanctifier doctor — environment sanity check".bold());
    println!();

    let checks = vec![
        check_rust(),
        check_soroban_cli(),
        check_z3(),
        check_cargo_expand(),
    ];

    let mut all_passed = true;
    for check in &checks {
        let icon = if check.passed {
            "✓".green().bold()
        } else {
            "✗".red().bold()
        };

        println!("  {} {}", icon, check.name);
        if args.verbose || !check.passed {
            println!("    {}", check.detail.dimmed());
        }
        if !check.passed {
            all_passed = false;
        }
    }

    println!();
    if all_passed {
        println!("{}", "All checks passed. Your environment is ready.".green());
    } else {
        println!(
            "{}",
            "Some checks failed. Fix the issues above before running sanctifier.".yellow()
        );
    }

    Ok(())
}

fn check_rust() -> CheckResult {
    match Command::new("rustc").arg("--version").output() {
        Ok(out) if out.status.success() => {
            let version = String::from_utf8_lossy(&out.stdout).trim().to_string();
            CheckResult {
                name: "Rust (rustc)",
                passed: true,
                detail: version,
            }
        }
        _ => CheckResult {
            name: "Rust (rustc)",
            passed: false,
            detail: "rustc not found — install Rust via https://rustup.rs".to_string(),
        },
    }
}

fn check_soroban_cli() -> CheckResult {
    // Soroban CLI may be installed as `soroban` or `stellar`
    for bin in &["stellar", "soroban"] {
        if let Ok(out) = Command::new(bin).arg("--version").output() {
            if out.status.success() {
                let version = String::from_utf8_lossy(&out.stdout).trim().to_string();
                return CheckResult {
                    name: "Soroban / Stellar CLI",
                    passed: true,
                    detail: format!("{bin}: {version}"),
                };
            }
        }
    }
    CheckResult {
        name: "Soroban / Stellar CLI",
        passed: false,
        detail:
            "Neither `stellar` nor `soroban` found — install via `cargo install stellar-cli`"
                .to_string(),
    }
}

fn check_z3() -> CheckResult {
    match Command::new("z3").arg("--version").output() {
        Ok(out) if out.status.success() => {
            let version = String::from_utf8_lossy(&out.stdout).trim().to_string();
            CheckResult {
                name: "Z3 SMT solver",
                passed: true,
                detail: version,
            }
        }
        _ => CheckResult {
            name: "Z3 SMT solver",
            passed: false,
            detail:
                "z3 not found — install via your package manager or https://github.com/Z3Prover/z3"
                    .to_string(),
        },
    }
}

fn check_cargo_expand() -> CheckResult {
    match Command::new("cargo").args(["expand", "--version"]).output() {
        Ok(out) if out.status.success() => {
            let version = String::from_utf8_lossy(&out.stdout).trim().to_string();
            CheckResult {
                name: "cargo-expand",
                passed: true,
                detail: version,
            }
        }
        _ => CheckResult {
            name: "cargo-expand",
            passed: false,
            detail: "cargo-expand not found — install via `cargo install cargo-expand`"
                .to_string(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_rust_returns_result() {
        let r = check_rust();
        assert_eq!(r.name, "Rust (rustc)");
        // We're running inside a Rust build environment, so it should pass.
        assert!(r.passed, "rustc must be present in the build environment");
        assert!(!r.detail.is_empty());
    }

    #[test]
    fn check_z3_returns_result() {
        let r = check_z3();
        assert_eq!(r.name, "Z3 SMT solver");
        // May or may not be installed; just confirm a non-empty detail is returned.
        assert!(!r.detail.is_empty());
    }

    #[test]
    fn check_soroban_cli_returns_result() {
        let r = check_soroban_cli();
        assert_eq!(r.name, "Soroban / Stellar CLI");
        assert!(!r.detail.is_empty());
    }

    #[test]
    fn check_cargo_expand_returns_result() {
        let r = check_cargo_expand();
        assert_eq!(r.name, "cargo-expand");
        assert!(!r.detail.is_empty());
    }
}
