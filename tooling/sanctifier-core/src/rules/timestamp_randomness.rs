//! Rule S029 — timestamp used as randomness source.
//!
//! Block timestamps are not secret entropy. Validators can nudge
//! `env.ledger().timestamp()` within a small window, making any
//! randomness derived from it manipulable.
//!
//! This rule fires when `env.ledger().timestamp()` appears inside:
//! - a function whose name contains `rand`, `seed`, `pick`, or `winner`, OR
//! - a variable binding whose name contains `rand`, `seed`, `pick`, or `winner`.

use super::{Rule, RuleViolation, Severity};
use syn::spanned::Spanned;
use syn::{parse_str, File, Item, Pat};

const SENSITIVE_NAMES: &[&str] = &["rand", "seed", "pick", "winner"];

pub struct TimestampRandomnessRule;

impl TimestampRandomnessRule {
    pub fn new() -> Self {
        Self
    }
}

impl Default for TimestampRandomnessRule {
    fn default() -> Self {
        Self::new()
    }
}

impl Rule for TimestampRandomnessRule {
    fn name(&self) -> &str {
        "timestamp_randomness"
    }

    fn description(&self) -> &str {
        "Detects env.ledger().timestamp() used as entropy for randomness in rand/seed/pick/winner expressions"
    }

    fn check(&self, source: &str) -> Vec<RuleViolation> {
        let file = match parse_str::<File>(source) {
            Ok(f) => f,
            Err(_) => return vec![],
        };

        let mut violations = Vec::new();

        for item in &file.items {
            if let Item::Impl(impl_block) = item {
                for impl_item in &impl_block.items {
                    if let syn::ImplItem::Fn(f) = impl_item {
                        let fn_name = f.sig.ident.to_string();
                        let fn_name_lower = fn_name.to_lowercase();
                        let fn_is_sensitive =
                            SENSITIVE_NAMES.iter().any(|kw| fn_name_lower.contains(kw));

                        let mut findings: Vec<(String, String)> = Vec::new();
                        scan_block(&f.block, &fn_name, fn_is_sensitive, &mut findings);

                        for (location, context) in findings {
                            violations.push(
                                RuleViolation::new(
                                    self.name(),
                                    Severity::Error,
                                    format!(
                                        "'{context}' uses `env.ledger().timestamp()` as randomness entropy. \
                                        Block timestamps are manipulable by validators and must not be used as a sole source of randomness.",
                                    ),
                                    location,
                                )
                                .with_suggestion(
                                    "Replace timestamp-based entropy with a VRF oracle or a combination \
                                    of unpredictable sources (e.g. transaction hash + sequence number). \
                                    See docs/rules/unsafe-prng.md (S029) for guidance."
                                        .to_string(),
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

/// Walk a block and collect (location, context_label) for every
/// `env.ledger().timestamp()` call that appears in a sensitive context.
fn scan_block(
    block: &syn::Block,
    fn_name: &str,
    fn_is_sensitive: bool,
    findings: &mut Vec<(String, String)>,
) {
    for stmt in &block.stmts {
        match stmt {
            syn::Stmt::Local(local) => {
                // Check if the bound variable name is sensitive.
                let var_is_sensitive = binding_name_is_sensitive(&local.pat);

                if let Some(init) = &local.init {
                    let sensitive = fn_is_sensitive || var_is_sensitive;
                    scan_expr(&init.expr, fn_name, sensitive, findings);
                }
            }
            syn::Stmt::Expr(expr, _) => {
                scan_expr(expr, fn_name, fn_is_sensitive, findings);
            }
            _ => {}
        }
    }
}

/// Return true if any identifier in the pattern contains a sensitive keyword.
fn binding_name_is_sensitive(pat: &Pat) -> bool {
    match pat {
        Pat::Ident(p) => {
            let name = p.ident.to_string().to_lowercase();
            SENSITIVE_NAMES.iter().any(|kw| name.contains(kw))
        }
        Pat::Type(p) => binding_name_is_sensitive(&p.pat),
        Pat::Tuple(t) => t.elems.iter().any(binding_name_is_sensitive),
        Pat::TupleStruct(ts) => ts.elems.iter().any(binding_name_is_sensitive),
        _ => false,
    }
}

/// Recursively scan an expression for `env.ledger().timestamp()` calls
/// when in a sensitive context.
fn scan_expr(
    expr: &syn::Expr,
    fn_name: &str,
    sensitive: bool,
    findings: &mut Vec<(String, String)>,
) {
    match expr {
        syn::Expr::MethodCall(m) => {
            let method = m.method.to_string();

            if method == "timestamp" && is_ledger_receiver(&m.receiver) {
                if sensitive {
                    let span = m.span();
                    findings.push((
                        format!("{}:line {}", fn_name, span.start().line),
                        fn_name.to_string(),
                    ));
                }
                // No need to recurse into the receiver — we've matched.
                return;
            }

            // Propagate sensitivity for method chains: if the call itself has
            // a sensitive name, the arguments inherit that sensitivity.
            let call_is_sensitive = sensitive
                || SENSITIVE_NAMES
                    .iter()
                    .any(|kw| method.to_lowercase().contains(kw));

            scan_expr(&m.receiver, fn_name, call_is_sensitive, findings);
            for arg in &m.args {
                scan_expr(arg, fn_name, call_is_sensitive, findings);
            }
        }
        syn::Expr::Call(c) => {
            // Check for path-style calls whose name is sensitive.
            let call_is_sensitive = sensitive || call_path_is_sensitive(&c.func);
            for arg in &c.args {
                scan_expr(arg, fn_name, call_is_sensitive, findings);
            }
            scan_expr(&c.func, fn_name, call_is_sensitive, findings);
        }
        syn::Expr::Assign(a) => {
            // Check if the left-hand side is a sensitive name.
            let lhs_sensitive = expr_ident_is_sensitive(&a.left);
            scan_expr(&a.right, fn_name, sensitive || lhs_sensitive, findings);
        }
        syn::Expr::Binary(b) => {
            scan_expr(&b.left, fn_name, sensitive, findings);
            scan_expr(&b.right, fn_name, sensitive, findings);
        }
        syn::Expr::Cast(c) => {
            scan_expr(&c.expr, fn_name, sensitive, findings);
        }
        syn::Expr::Unary(u) => {
            scan_expr(&u.expr, fn_name, sensitive, findings);
        }
        syn::Expr::Paren(p) => {
            scan_expr(&p.expr, fn_name, sensitive, findings);
        }
        syn::Expr::Block(b) => {
            scan_block(&b.block, fn_name, sensitive, findings);
        }
        syn::Expr::If(i) => {
            scan_expr(&i.cond, fn_name, sensitive, findings);
            scan_block(&i.then_branch, fn_name, sensitive, findings);
            if let Some((_, else_expr)) = &i.else_branch {
                scan_expr(else_expr, fn_name, sensitive, findings);
            }
        }
        syn::Expr::Match(m) => {
            scan_expr(&m.expr, fn_name, sensitive, findings);
            for arm in &m.arms {
                scan_expr(&arm.body, fn_name, sensitive, findings);
            }
        }
        syn::Expr::ForLoop(f) => {
            scan_expr(&f.expr, fn_name, sensitive, findings);
            scan_block(&f.body, fn_name, sensitive, findings);
        }
        syn::Expr::While(w) => {
            scan_expr(&w.cond, fn_name, sensitive, findings);
            scan_block(&w.body, fn_name, sensitive, findings);
        }
        syn::Expr::Loop(l) => {
            scan_block(&l.body, fn_name, sensitive, findings);
        }
        syn::Expr::Return(r) => {
            if let Some(ret) = &r.expr {
                scan_expr(ret, fn_name, sensitive, findings);
            }
        }
        _ => {}
    }
}

/// Returns true when the expression looks like `<x>.ledger()`.
fn is_ledger_receiver(expr: &syn::Expr) -> bool {
    match expr {
        syn::Expr::MethodCall(m) => m.method == "ledger",
        syn::Expr::Paren(p) => is_ledger_receiver(&p.expr),
        _ => false,
    }
}

/// Returns true when a call's function path contains a sensitive keyword.
fn call_path_is_sensitive(func: &syn::Expr) -> bool {
    if let syn::Expr::Path(p) = func {
        if let Some(seg) = p.path.segments.last() {
            let name = seg.ident.to_string().to_lowercase();
            return SENSITIVE_NAMES.iter().any(|kw| name.contains(kw));
        }
    }
    false
}

/// Returns true when the expression is an identifier with a sensitive name.
fn expr_ident_is_sensitive(expr: &syn::Expr) -> bool {
    if let syn::Expr::Path(p) = expr {
        if let Some(seg) = p.path.segments.last() {
            let name = seg.ident.to_string().to_lowercase();
            return SENSITIVE_NAMES.iter().any(|kw| name.contains(kw));
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flags_timestamp_in_rand_function() {
        let rule = TimestampRandomnessRule::new();
        let source = r#"
            impl LotteryContract {
                pub fn pick_winner(env: Env, participants: Vec<Address>) -> Address {
                    let idx = env.ledger().timestamp() % participants.len() as u64;
                    participants.get(idx as u32).unwrap()
                }
            }
        "#;
        let violations = rule.check(source);
        assert!(
            !violations.is_empty(),
            "timestamp in pick_winner must be flagged"
        );
        assert!(violations[0].message.contains("pick_winner"));
    }

    #[test]
    fn flags_timestamp_assigned_to_seed_variable() {
        let rule = TimestampRandomnessRule::new();
        let source = r#"
            impl MyContract {
                pub fn initialize(env: Env) {
                    let seed = env.ledger().timestamp();
                    env.storage().persistent().set(&symbol_short!("seed"), &seed);
                }
            }
        "#;
        let violations = rule.check(source);
        assert!(
            !violations.is_empty(),
            "timestamp assigned to 'seed' must be flagged"
        );
    }

    #[test]
    fn flags_timestamp_assigned_to_rand_variable() {
        let rule = TimestampRandomnessRule::new();
        let source = r#"
            impl MyContract {
                pub fn roll(env: Env) -> u64 {
                    let rand = env.ledger().timestamp() % 6 + 1;
                    rand
                }
            }
        "#;
        let violations = rule.check(source);
        assert!(
            !violations.is_empty(),
            "timestamp assigned to 'rand' must be flagged"
        );
    }

    #[test]
    fn flags_timestamp_in_winner_function() {
        let rule = TimestampRandomnessRule::new();
        let source = r#"
            impl Lottery {
                pub fn draw_winner(env: Env) -> u64 {
                    env.ledger().timestamp() % 100
                }
            }
        "#;
        let violations = rule.check(source);
        assert!(
            !violations.is_empty(),
            "timestamp in draw_winner must be flagged"
        );
    }

    #[test]
    fn no_flag_for_timestamp_in_non_sensitive_function() {
        let rule = TimestampRandomnessRule::new();
        let source = r#"
            impl MyContract {
                pub fn check_expiry(env: Env, deadline: u64) -> bool {
                    env.ledger().timestamp() > deadline
                }
            }
        "#;
        let violations = rule.check(source);
        assert!(
            violations.is_empty(),
            "timestamp for time comparison must not be flagged"
        );
    }

    #[test]
    fn no_flag_when_no_timestamp_call() {
        let rule = TimestampRandomnessRule::new();
        let source = r#"
            impl MyContract {
                pub fn pick_winner(env: Env) -> u64 {
                    env.prng().gen_range(0..100)
                }
            }
        "#;
        let violations = rule.check(source);
        assert!(violations.is_empty(), "no timestamp means no flag");
    }

    #[test]
    fn empty_source_produces_no_findings() {
        let rule = TimestampRandomnessRule::new();
        assert!(rule.check("").is_empty());
    }

    #[test]
    fn invalid_source_produces_no_panic() {
        let rule = TimestampRandomnessRule::new();
        assert!(rule.check("not valid rust {{{{").is_empty());
    }

    #[test]
    fn finding_has_suggestion_linking_to_unsafe_prng_doc() {
        let rule = TimestampRandomnessRule::new();
        let source = r#"
            impl Raffle {
                pub fn pick_winner(env: Env) -> u32 {
                    (env.ledger().timestamp() % 10) as u32
                }
            }
        "#;
        let violations = rule.check(source);
        assert!(!violations.is_empty());
        let suggestion = violations[0].suggestion.as_deref().unwrap_or("");
        assert!(
            suggestion.contains("unsafe-prng.md") || suggestion.contains("S029"),
            "suggestion must link to unsafe-prng.md or S029"
        );
    }
}
