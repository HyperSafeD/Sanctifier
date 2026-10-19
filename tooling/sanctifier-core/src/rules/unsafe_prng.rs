use crate::rules::{Rule, RuleViolation, Severity};
use crate::soroban_v21::is_prng_function;
use syn::spanned::Spanned;
use syn::{parse_str, File, Item};

/// Rule that detects use of env.prng() without proper seeding in state-critical code.
///
/// Soroban's PRNG can be predictable if not properly seeded, which is dangerous
/// for security-critical operations like lottery systems, token distribution,
/// or any randomness-dependent logic that affects contract state.
pub struct UnsafePrngRule;

impl UnsafePrngRule {
    /// Create a new instance.
    pub fn new() -> Self {
        Self
    }
}

impl Default for UnsafePrngRule {
    fn default() -> Self {
        Self::new()
    }
}

impl Rule for UnsafePrngRule {
    fn name(&self) -> &str {
        "unsafe_prng"
    }

    fn description(&self) -> &str {
        "Detects use of env.prng() or PRNG functions without proper seeding in state-critical code"
    }

    fn check(&self, source: &str) -> Vec<RuleViolation> {
        let file = match parse_str::<File>(source) {
            Ok(f) => f,
            Err(_) => return vec![],
        };

        let mut violations = Vec::new();

        for item in &file.items {
            if let Item::Impl(i) = item {
                for impl_item in &i.items {
                    if let syn::ImplItem::Fn(f) = impl_item {
                        let fn_name = f.sig.ident.to_string();
                        let mut has_prng_usage = false;
                        let mut has_prng_reseed = false;
                        let mut has_storage_mutation = false;
                        let mut prng_locations = Vec::new();

                        // Analyze function body
                        analyze_fn_body(
                            &f.block,
                            &mut has_prng_usage,
                            &mut has_prng_reseed,
                            &mut has_storage_mutation,
                            &mut prng_locations,
                        );

                        // Flag if PRNG is used in a function that mutates storage without reseeding
                        if has_prng_usage && has_storage_mutation && !has_prng_reseed {
                            let location = if !prng_locations.is_empty() {
                                format!("{}:{}", fn_name, prng_locations[0])
                            } else {
                                fn_name.clone()
                            };

                            violations.push(
                                RuleViolation::new(
                                    self.name(),
                                    Severity::Warning,
                                    format!(
                                        "Function '{}' uses PRNG in state-critical code without explicit reseeding. \
                                        Predictable randomness can lead to security vulnerabilities.",
                                        fn_name
                                    ),
                                    location,
                                )
                                .with_suggestion(
                                    "Consider using prng.reseed() with unpredictable entropy (e.g., from ledger timestamp, \
                                    transaction hash, or external oracle) before generating random values for state-critical operations. \
                                    Alternatively, document why the default seeding is sufficient for your use case.".to_string()
                                ),
                            );
                        }
                    }
                }
            }
        }

        violations
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Recursively analyze function body for PRNG usage, reseeding, and storage mutations.
fn analyze_fn_body(
    block: &syn::Block,
    has_prng_usage: &mut bool,
    has_prng_reseed: &mut bool,
    has_storage_mutation: &mut bool,
    prng_locations: &mut Vec<String>,
) {
    for stmt in &block.stmts {
        match stmt {
            syn::Stmt::Expr(expr, _) => {
                analyze_expr(
                    expr,
                    has_prng_usage,
                    has_prng_reseed,
                    has_storage_mutation,
                    prng_locations,
                );
            }
            syn::Stmt::Local(local) => {
                if let Some(init) = &local.init {
                    analyze_expr(
                        &init.expr,
                        has_prng_usage,
                        has_prng_reseed,
                        has_storage_mutation,
                        prng_locations,
                    );
                }
            }
            _ => {}
        }
    }
}

/// Recursively analyze expressions for PRNG patterns and storage mutations.
fn analyze_expr(
    expr: &syn::Expr,
    has_prng_usage: &mut bool,
    has_prng_reseed: &mut bool,
    has_storage_mutation: &mut bool,
    prng_locations: &mut Vec<String>,
) {
    match expr {
        syn::Expr::MethodCall(m) => {
            let method_name = m.method.to_string();

            // Check for PRNG method calls
            if method_name == "prng" {
                *has_prng_usage = true;
                let span = m.span();
                prng_locations.push(format!("line {}", span.start().line));
            }

            // Check for PRNG host functions (v21)
            if is_prng_function(&method_name) {
                *has_prng_usage = true;
                let span = m.span();
                prng_locations.push(format!("line {}", span.start().line));
            }

            // Check for reseed calls
            if method_name == "reseed" {
                let receiver_str = quote::quote!(#m.receiver).to_string();
                if receiver_str.contains("prng") || receiver_str.contains("Prng") {
                    *has_prng_reseed = true;
                }
            }

            // Check for storage mutations
            if is_storage_mutation(&method_name) {
                let receiver_str = quote::quote!(#m.receiver).to_string();
                if receiver_str.contains("storage")
                    || receiver_str.contains("persistent")
                    || receiver_str.contains("temporary")
                    || receiver_str.contains("instance")
                {
                    *has_storage_mutation = true;
                }
            }

            // Recurse into receiver and arguments
            analyze_expr(
                &m.receiver,
                has_prng_usage,
                has_prng_reseed,
                has_storage_mutation,
                prng_locations,
            );
            for arg in &m.args {
                analyze_expr(
                    arg,
                    has_prng_usage,
                    has_prng_reseed,
                    has_storage_mutation,
                    prng_locations,
                );
            }
        }
        syn::Expr::Call(c) => {
            // Check for PRNG function calls
            if let syn::Expr::Path(p) = &*c.func {
                if let Some(segment) = p.path.segments.last() {
                    let ident = segment.ident.to_string();
                    if is_prng_function(&ident) {
                        *has_prng_usage = true;
                        let span = c.span();
                        prng_locations.push(format!("line {}", span.start().line));
                    }
                }
            }

            for arg in &c.args {
                analyze_expr(
                    arg,
                    has_prng_usage,
                    has_prng_reseed,
                    has_storage_mutation,
                    prng_locations,
                );
            }
        }
        syn::Expr::Block(b) => {
            analyze_fn_body(
                &b.block,
                has_prng_usage,
                has_prng_reseed,
                has_storage_mutation,
                prng_locations,
            );
        }
        syn::Expr::If(i) => {
            analyze_expr(
                &i.cond,
                has_prng_usage,
                has_prng_reseed,
                has_storage_mutation,
                prng_locations,
            );
            analyze_fn_body(
                &i.then_branch,
                has_prng_usage,
                has_prng_reseed,
                has_storage_mutation,
                prng_locations,
            );
            if let Some((_, else_expr)) = &i.else_branch {
                analyze_expr(
                    else_expr,
                    has_prng_usage,
                    has_prng_reseed,
                    has_storage_mutation,
                    prng_locations,
                );
            }
        }
        syn::Expr::Match(m) => {
            analyze_expr(
                &m.expr,
                has_prng_usage,
                has_prng_reseed,
                has_storage_mutation,
                prng_locations,
            );
            for arm in &m.arms {
                analyze_expr(
                    &arm.body,
                    has_prng_usage,
                    has_prng_reseed,
                    has_storage_mutation,
                    prng_locations,
                );
            }
        }
        syn::Expr::ForLoop(f) => {
            analyze_expr(
                &f.expr,
                has_prng_usage,
                has_prng_reseed,
                has_storage_mutation,
                prng_locations,
            );
            analyze_fn_body(
                &f.body,
                has_prng_usage,
                has_prng_reseed,
                has_storage_mutation,
                prng_locations,
            );
        }
        syn::Expr::While(w) => {
            analyze_expr(
                &w.cond,
                has_prng_usage,
                has_prng_reseed,
                has_storage_mutation,
                prng_locations,
            );
            analyze_fn_body(
                &w.body,
                has_prng_usage,
                has_prng_reseed,
                has_storage_mutation,
                prng_locations,
            );
        }
        syn::Expr::Loop(l) => {
            analyze_fn_body(
                &l.body,
                has_prng_usage,
                has_prng_reseed,
                has_storage_mutation,
                prng_locations,
            );
        }
        _ => {}
    }
}

/// Check if a method name indicates storage mutation.
fn is_storage_mutation(method_name: &str) -> bool {
    matches!(
        method_name,
        "set" | "update" | "remove" | "extend_ttl" | "bump"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flags_prng_usage_with_storage_mutation_without_reseed() {
        let rule = UnsafePrngRule::new();
        let source = r#"
            impl LotteryContract {
                pub fn draw_winner(env: Env) -> Address {
                    let random_index = env.prng().u64_in_range(0..100);
                    let winner = get_participant(random_index);
                    env.storage().persistent().set(&symbol_short!("winner"), &winner);
                    winner
                }
            }
        "#;
        let violations = rule.check(source);
        assert!(
            !violations.is_empty(),
            "PRNG without reseed should be flagged"
        );
        assert!(violations[0].message.contains("draw_winner"));
        assert!(violations[0].message.contains("without explicit reseeding"));
    }

    #[test]
    fn no_violation_when_prng_reseeded() {
        let rule = UnsafePrngRule::new();
        let source = r#"
            impl LotteryContract {
                pub fn draw_winner(env: Env) -> Address {
                    let mut prng = env.prng();
                    prng.reseed(env.ledger().timestamp());
                    let random_index = prng.u64_in_range(0..100);
                    let winner = get_participant(random_index);
                    env.storage().persistent().set(&symbol_short!("winner"), &winner);
                    winner
                }
            }
        "#;
        let violations = rule.check(source);
        assert!(
            violations.is_empty(),
            "PRNG with reseed should not be flagged"
        );
    }

    #[test]
    fn no_violation_when_prng_without_storage_mutation() {
        let rule = UnsafePrngRule::new();
        let source = r#"
            impl MyContract {
                pub fn get_random_value(env: Env) -> u64 {
                    env.prng().u64_in_range(0..100)
                }
            }
        "#;
        let violations = rule.check(source);
        assert!(
            violations.is_empty(),
            "PRNG without storage mutation should not be flagged"
        );
    }

    #[test]
    fn no_violation_when_only_storage_mutation() {
        let rule = UnsafePrngRule::new();
        let source = r#"
            impl MyContract {
                pub fn set_value(env: Env, value: u64) {
                    env.storage().persistent().set(&symbol_short!("value"), &value);
                }
            }
        "#;
        let violations = rule.check(source);
        assert!(
            violations.is_empty(),
            "Storage mutation without PRNG should not be flagged"
        );
    }

    #[test]
    fn flags_v21_prng_host_functions() {
        let rule = UnsafePrngRule::new();
        let source = r#"
            impl MyContract {
                pub fn shuffle_and_store(env: Env, items: Vec<u32>) {
                    let shuffled = env.prng_vec_shuffle(items);
                    env.storage().persistent().set(&symbol_short!("items"), &shuffled);
                }
            }
        "#;
        let violations = rule.check(source);
        assert!(
            !violations.is_empty(),
            "v21 PRNG functions should be flagged"
        );
    }

    #[test]
    fn empty_source_produces_no_findings() {
        let rule = UnsafePrngRule::new();
        let violations = rule.check("");
        assert!(
            violations.is_empty(),
            "empty source must produce no findings"
        );
    }

    #[test]
    fn invalid_source_produces_no_panic() {
        let rule = UnsafePrngRule::new();
        let violations = rule.check("not valid rust {{{{");
        assert!(
            violations.is_empty(),
            "parse error must return empty, not panic"
        );
    }
}
