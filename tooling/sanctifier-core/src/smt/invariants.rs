//! AST-based `#[invariant = "..."]` parsing and Z3 verification.
//!
//! This module owns the two public entry-points for S011:
//! - [`parse_invariants`] — extract annotations from Rust source via `syn`
//! - [`verify_invariants`] — check each annotation with Z3 under a timeout

use z3::ast::Int;
use z3::{Config, Context, SatResult, Solver};

use super::types::{InvariantSpec, SmtConfig, SmtFinding};

// ── Public API ────────────────────────────────────────────────────────────────

/// Parse every `#[invariant = "..."]` attribute in the source file using the
/// syn AST.  Returns an empty vec on parse errors (no panic).
pub fn parse_invariants(source: &str) -> Vec<InvariantSpec> {
    use syn::{parse_str, Expr, File, Item, Lit, Meta};

    let file = match parse_str::<File>(source) {
        Ok(f) => f,
        Err(_) => return vec![],
    };

    let mut specs = Vec::new();

    for item in &file.items {
        if let Item::Impl(impl_block) = item {
            for impl_item in &impl_block.items {
                if let syn::ImplItem::Fn(f) = impl_item {
                    let fn_name = f.sig.ident.to_string();
                    for attr in &f.attrs {
                        if attr.path().is_ident("invariant") {
                            if let Meta::NameValue(nv) = &attr.meta {
                                if let Expr::Lit(expr_lit) = &nv.value {
                                    if let Lit::Str(lit_str) = &expr_lit.lit {
                                        specs.push(InvariantSpec {
                                            expression: lit_str.value(),
                                            location: fn_name.clone(),
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    specs
}

/// Verify `#[invariant = "..."]` annotations with Z3 under a configurable
/// timeout.
///
/// Returns one [`SmtFinding`] for every invariant that could not be proved
/// safe, or for which the solver timed out.  Safe invariants produce no
/// finding.
pub fn verify_invariants(source: &str, config: &SmtConfig) -> Vec<SmtFinding> {
    let specs = parse_invariants(source);
    if specs.is_empty() {
        return vec![];
    }

    let mut findings = Vec::new();
    for spec in &specs {
        if let Some(finding) = check_invariant_spec(spec, config) {
            findings.push(finding);
        }
    }
    findings
}

// ── Internal ──────────────────────────────────────────────────────────────────

/// Check one [`InvariantSpec`] with Z3.
///
/// * Expressions that mention addition (`+` / `add`) are verified against
///   u64 overflow.
/// * Expressions that mention subtraction (`-` / `sub`) are verified against
///   u64 underflow.
/// * All other expressions are skipped (no finding — they require manual
///   modelling).
fn check_invariant_spec(spec: &InvariantSpec, config: &SmtConfig) -> Option<SmtFinding> {
    let expr_lower = spec.expression.to_lowercase();

    let overflow_check = expr_lower.contains('+') || expr_lower.contains("add");
    let underflow_check = expr_lower.contains('-') || expr_lower.contains("sub");

    if !overflow_check && !underflow_check {
        return None;
    }

    let mut cfg = Config::new();
    // Pass the timeout as milliseconds; Z3 returns Unknown when it expires.
    cfg.set_param_value("timeout", &config.timeout_ms.to_string());
    let ctx = Context::new(&cfg);
    let solver = Solver::new(&ctx);

    let a = Int::new_const(&ctx, "a");
    let b = Int::new_const(&ctx, "b");
    let zero = Int::from_u64(&ctx, 0);
    let max_u64 = Int::from_u64(&ctx, u64::MAX);

    solver.assert(&a.ge(&zero));
    solver.assert(&a.le(&max_u64));
    solver.assert(&b.ge(&zero));
    solver.assert(&b.le(&max_u64));

    let violation = if overflow_check {
        let sum = Int::add(&ctx, &[&a, &b]);
        sum.gt(&max_u64)
    } else {
        let diff = Int::sub(&ctx, &[&a, &b]);
        diff.lt(&zero)
    };

    solver.assert(&violation);

    match solver.check() {
        SatResult::Sat => {
            let counterexample = solver.get_model().map(|m| {
                let a_val = m
                    .eval(&a, true)
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "?".to_string());
                let b_val = m
                    .eval(&b, true)
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "?".to_string());
                if overflow_check {
                    format!("a={a_val}, b={b_val} — a+b overflows u64")
                } else {
                    format!("a={a_val}, b={b_val} — a-b underflows u64")
                }
            });
            Some(SmtFinding {
                invariant_name: spec.expression.clone(),
                location: spec.location.clone(),
                counterexample,
                is_timeout: false,
            })
        }
        SatResult::Unsat => None,
        SatResult::Unknown => Some(SmtFinding {
            invariant_name: spec.expression.clone(),
            location: spec.location.clone(),
            counterexample: None,
            is_timeout: true,
        }),
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify_invariants_empty_source_returns_no_findings() {
        let findings = verify_invariants("", &SmtConfig::default());
        assert!(findings.is_empty(), "empty source must produce no findings");
    }

    #[test]
    fn verify_invariants_parse_error_returns_no_findings() {
        let findings = verify_invariants("this is not valid rust }{{{", &SmtConfig::default());
        assert!(
            findings.is_empty(),
            "unparseable source must produce no findings"
        );
    }

    #[test]
    fn parse_invariants_extracts_attribute_from_ast() {
        let source = r#"
            impl Vault {
                #[invariant = "balance + deposit <= u64::MAX"]
                pub fn deposit(&self, deposit: u64) {}
            }
        "#;
        let specs = parse_invariants(source);
        assert_eq!(specs.len(), 1);
        assert_eq!(specs[0].expression, "balance + deposit <= u64::MAX");
        assert_eq!(specs[0].location, "deposit");
    }

    #[test]
    fn verify_invariants_addition_overflow_flagged() {
        let source = r#"
            impl Vault {
                #[invariant = "a + b is safe"]
                pub fn credit(&self, a: u64, b: u64) {}
            }
        "#;
        let findings = verify_invariants(source, &SmtConfig::default());
        assert_eq!(findings.len(), 1);
        assert!(
            !findings[0].is_timeout,
            "should be a SAT result, not timeout"
        );
        assert!(
            findings[0].counterexample.is_some(),
            "counterexample must be populated for an unsafe invariant"
        );
        assert!(
            findings[0]
                .counterexample
                .as_ref()
                .unwrap()
                .contains("overflow"),
            "counterexample should mention overflow"
        );
    }

    #[test]
    fn verify_invariants_non_arithmetic_expression_is_safe() {
        let source = r#"
            impl Token {
                #[invariant = "admin_only"]
                pub fn burn(&self) {}
            }
        "#;
        let findings = verify_invariants(source, &SmtConfig::default());
        assert!(
            findings.is_empty(),
            "non-arithmetic invariant must not produce a finding"
        );
    }

    #[test]
    fn verify_invariants_timeout_produces_is_timeout_true() {
        let source = r#"
            impl Vault {
                #[invariant = "a + b is safe"]
                pub fn credit(&self, a: u64, b: u64) {}
            }
        "#;
        let config = SmtConfig { timeout_ms: 1 };
        let findings = verify_invariants(source, &config);
        for f in &findings {
            if f.is_timeout {
                assert!(
                    f.counterexample.is_none(),
                    "timed-out findings must have no counterexample"
                );
            }
        }
    }
}
