//! Rule S031 — gas exhaustion risk from unbounded user-controlled loops.
//!
//! Soroban transactions have a fixed gas/instruction budget. A loop whose
//! iteration count grows with a caller-supplied collection (`Vec<T>`,
//! `Map<K, V>`, `Bytes`) or a raw integer parameter has no upper bound unless
//! the contract clamps it — so a sufficiently large input reverts the
//! transaction out-of-gas instead of failing gracefully (or "bricking" the
//! call for every other caller sharing the same budget).
//!
//! This rule flags `for`/`while` loops inside `impl` methods whose bound
//! traces directly back to such a parameter, unless the bound expression
//! itself is clamped via `.min(...)`/`.saturating_sub(...)`/`.take(...)`.

use super::{Rule, RuleViolation, Severity};
use syn::spanned::Spanned;
use syn::{parse_str, File, Item};

pub struct GasExhaustionRiskRule;

impl GasExhaustionRiskRule {
    pub fn new() -> Self {
        Self
    }
}

impl Default for GasExhaustionRiskRule {
    fn default() -> Self {
        Self::new()
    }
}

impl Rule for GasExhaustionRiskRule {
    fn name(&self) -> &str {
        "gas_exhaustion_risk"
    }

    fn description(&self) -> &str {
        "Detects loops whose iteration count derives from an unbounded user-controlled parameter (S031)"
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
                        let unbounded_params = unbounded_param_names(&f.sig);
                        if unbounded_params.is_empty() {
                            continue;
                        }

                        let mut findings = Vec::new();
                        scan_block(&f.block, &fn_name, &unbounded_params, &mut findings);

                        for (location, reason) in findings {
                            violations.push(
                                RuleViolation::new(
                                    self.name(),
                                    Severity::Warning,
                                    format!(
                                        "Loop in '{fn_name}' {reason}, which can exhaust the gas budget \
                                        and cause an out-of-gas revert (S031)."
                                    ),
                                    location,
                                )
                                .with_suggestion(
                                    "Cap the iteration count with a fixed maximum, e.g. `.iter().take(MAX_ITEMS)` \
                                    or an explicit `if param.len() > MAX_ITEMS { return Err(...) }` check before \
                                    the loop, so gas cost cannot scale unbounded with caller-supplied input."
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

/// Returns the names of function parameters whose type makes their size
/// directly caller-controlled: `Vec<_>`, `Map<_, _>`, `Bytes`, or a raw
/// unsigned integer (`u32`, `u64`, `u128`, `usize`) that could be used
/// directly as a loop bound.
fn unbounded_param_names(sig: &syn::Signature) -> Vec<String> {
    sig.inputs
        .iter()
        .filter_map(|arg| {
            let syn::FnArg::Typed(pt) = arg else {
                return None;
            };
            let syn::Pat::Ident(pi) = pt.pat.as_ref() else {
                return None;
            };
            if type_is_unbounded(pt.ty.as_ref()) {
                Some(pi.ident.to_string())
            } else {
                None
            }
        })
        .collect()
}

fn type_is_unbounded(ty: &syn::Type) -> bool {
    match ty {
        syn::Type::Path(tp) => {
            let Some(seg) = tp.path.segments.last() else {
                return false;
            };
            matches!(
                seg.ident.to_string().as_str(),
                "Vec" | "Map" | "Bytes" | "u32" | "u64" | "u128" | "usize" | "i128"
            )
        }
        syn::Type::Reference(r) => type_is_unbounded(r.elem.as_ref()),
        _ => false,
    }
}

/// Walk a block looking for `for`/`while` loops whose bound traces back to
/// an unbounded parameter.
fn scan_block(
    block: &syn::Block,
    fn_name: &str,
    unbounded_params: &[String],
    findings: &mut Vec<(String, String)>,
) {
    for stmt in &block.stmts {
        match stmt {
            syn::Stmt::Expr(expr, _) => scan_expr(expr, fn_name, unbounded_params, findings),
            syn::Stmt::Local(local) => {
                if let Some(init) = &local.init {
                    scan_expr(&init.expr, fn_name, unbounded_params, findings);
                }
            }
            _ => {}
        }
    }
}

fn scan_expr(
    expr: &syn::Expr,
    fn_name: &str,
    unbounded_params: &[String],
    findings: &mut Vec<(String, String)>,
) {
    match expr {
        syn::Expr::ForLoop(f) => {
            if let Some(reason) = unbounded_iterator_reason(&f.expr, unbounded_params) {
                let line = f.span().start().line;
                findings.push((format!("{fn_name}:line {line}"), reason));
            }
            scan_block(&f.body, fn_name, unbounded_params, findings);
        }
        syn::Expr::While(w) => {
            if let Some(reason) = unbounded_condition_reason(&w.cond, unbounded_params) {
                let line = w.span().start().line;
                findings.push((format!("{fn_name}:line {line}"), reason));
            }
            scan_block(&w.body, fn_name, unbounded_params, findings);
        }
        syn::Expr::Loop(l) => scan_block(&l.body, fn_name, unbounded_params, findings),
        syn::Expr::Block(b) => scan_block(&b.block, fn_name, unbounded_params, findings),
        syn::Expr::If(i) => {
            scan_block(&i.then_branch, fn_name, unbounded_params, findings);
            if let Some((_, else_expr)) = &i.else_branch {
                scan_expr(else_expr, fn_name, unbounded_params, findings);
            }
        }
        syn::Expr::Match(m) => {
            for arm in &m.arms {
                scan_expr(&arm.body, fn_name, unbounded_params, findings);
            }
        }
        _ => {}
    }
}

/// If `iter_expr` (the expression after `in` in a `for x in <iter_expr>`)
/// iterates the full length of an unbounded parameter without a clamp,
/// returns a human-readable reason. Otherwise `None`.
fn unbounded_iterator_reason(iter_expr: &syn::Expr, unbounded_params: &[String]) -> Option<String> {
    if expr_contains_clamp(iter_expr) {
        return None;
    }
    match iter_expr {
        // `param.iter()`, `param.iter().rev()`, `(&param).iter()`, etc.
        syn::Expr::MethodCall(m) => {
            if let Some(param) = root_receiver_param(&m.receiver, unbounded_params) {
                return Some(format!("iterates the full length of parameter '{param}'"));
            }
            unbounded_iterator_reason(&m.receiver, unbounded_params)
        }
        syn::Expr::Reference(r) => unbounded_iterator_reason(&r.expr, unbounded_params),
        syn::Expr::Paren(p) => unbounded_iterator_reason(&p.expr, unbounded_params),
        // `0..param` or `0..param.len()`
        syn::Expr::Range(r) => {
            let end = r.end.as_deref()?;
            range_bound_param(end, unbounded_params)
                .map(|param| format!("iterates up to bound derived from parameter '{param}'"))
        }
        _ => None,
    }
}

/// If `cond` (a `while` condition) compares a loop counter against an
/// unbounded parameter (or its `.len()`) without a clamp, returns a reason.
fn unbounded_condition_reason(cond: &syn::Expr, unbounded_params: &[String]) -> Option<String> {
    if expr_contains_clamp(cond) {
        return None;
    }
    let syn::Expr::Binary(b) = cond else {
        return None;
    };
    if !matches!(
        b.op,
        syn::BinOp::Lt(_) | syn::BinOp::Le(_) | syn::BinOp::Ne(_)
    ) {
        return None;
    }
    range_bound_param(&b.right, unbounded_params)
        .or_else(|| range_bound_param(&b.left, unbounded_params))
        .map(|param| format!("loop condition is bounded by parameter '{param}'"))
}

/// Returns the parameter name if `expr` is `<param>` or `<param>.len()`.
fn range_bound_param(expr: &syn::Expr, unbounded_params: &[String]) -> Option<String> {
    match expr {
        syn::Expr::Path(p) => {
            let name = p.path.segments.last()?.ident.to_string();
            unbounded_params.contains(&name).then_some(name)
        }
        syn::Expr::MethodCall(m) if m.method == "len" => {
            root_receiver_param(&m.receiver, unbounded_params)
        }
        syn::Expr::Paren(p) => range_bound_param(&p.expr, unbounded_params),
        _ => None,
    }
}

/// Walks through a receiver chain (`&param`, `(*param)`, etc.) to find a
/// root identifier matching one of `unbounded_params`.
fn root_receiver_param(expr: &syn::Expr, unbounded_params: &[String]) -> Option<String> {
    match expr {
        syn::Expr::Path(p) => {
            let name = p.path.segments.last()?.ident.to_string();
            unbounded_params.contains(&name).then_some(name)
        }
        syn::Expr::Reference(r) => root_receiver_param(&r.expr, unbounded_params),
        syn::Expr::Unary(u) => root_receiver_param(&u.expr, unbounded_params),
        syn::Expr::Paren(p) => root_receiver_param(&p.expr, unbounded_params),
        _ => None,
    }
}

/// Returns true if `expr` contains a clamp-style call (`.min(`, `.take(`,
/// `.saturating_sub(`, `.truncate(`) anywhere in its method-call chain,
/// signalling the author already bounded the iteration count.
fn expr_contains_clamp(expr: &syn::Expr) -> bool {
    match expr {
        syn::Expr::MethodCall(m) => {
            let method = m.method.to_string();
            matches!(
                method.as_str(),
                "min" | "take" | "saturating_sub" | "truncate" | "clamp"
            ) || expr_contains_clamp(&m.receiver)
        }
        syn::Expr::Reference(r) => expr_contains_clamp(&r.expr),
        syn::Expr::Paren(p) => expr_contains_clamp(&p.expr),
        syn::Expr::Range(r) => r.end.as_deref().map(expr_contains_clamp).unwrap_or(false),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flags_for_loop_over_full_vec_param() {
        let rule = GasExhaustionRiskRule::new();
        let source = r#"
            impl Batch {
                pub fn process(env: Env, recipients: Vec<Address>) {
                    for r in recipients.iter() {
                        do_transfer(&env, r);
                    }
                }
            }
        "#;
        let violations = rule.check(source);
        assert!(
            !violations.is_empty(),
            "unbounded Vec parameter loop must be flagged"
        );
        assert!(violations[0].message.contains("process"));
    }

    #[test]
    fn flags_range_loop_bound_by_integer_param() {
        let rule = GasExhaustionRiskRule::new();
        let source = r#"
            impl Counter {
                pub fn bump_many(env: Env, count: u32) {
                    for i in 0..count {
                        bump(&env, i);
                    }
                }
            }
        "#;
        let violations = rule.check(source);
        assert!(
            !violations.is_empty(),
            "range bound by raw integer parameter must be flagged"
        );
    }

    #[test]
    fn flags_while_loop_bound_by_param_len() {
        let rule = GasExhaustionRiskRule::new();
        let source = r#"
            impl Batch {
                pub fn process(env: Env, items: Vec<u32>) {
                    let mut i = 0;
                    while i < items.len() {
                        i += 1;
                    }
                }
            }
        "#;
        let violations = rule.check(source);
        assert!(
            !violations.is_empty(),
            "while loop bound by param.len() must be flagged"
        );
    }

    #[test]
    fn does_not_flag_when_take_clamp_applied() {
        let rule = GasExhaustionRiskRule::new();
        let source = r#"
            impl Batch {
                pub fn process(env: Env, recipients: Vec<Address>) {
                    for r in recipients.iter().take(MAX_BATCH) {
                        do_transfer(&env, r);
                    }
                }
            }
        "#;
        let violations = rule.check(source);
        assert!(
            violations.is_empty(),
            ".take(...) clamp must suppress the finding"
        );
    }

    #[test]
    fn does_not_flag_when_min_clamp_applied_to_range() {
        let rule = GasExhaustionRiskRule::new();
        let source = r#"
            impl Counter {
                pub fn bump_many(env: Env, count: u32) {
                    for i in 0..count.min(MAX_COUNT) {
                        bump(&env, i);
                    }
                }
            }
        "#;
        let violations = rule.check(source);
        assert!(
            violations.is_empty(),
            ".min(...) clamp on the range bound must suppress the finding"
        );
    }

    #[test]
    fn does_not_flag_loop_over_fixed_constant() {
        let rule = GasExhaustionRiskRule::new();
        let source = r#"
            impl Counter {
                pub fn bump_fixed(env: Env) {
                    for i in 0..10 {
                        bump(&env, i);
                    }
                }
            }
        "#;
        let violations = rule.check(source);
        assert!(
            violations.is_empty(),
            "loop bound by a fixed literal must not be flagged"
        );
    }

    #[test]
    fn does_not_flag_function_without_unbounded_params() {
        let rule = GasExhaustionRiskRule::new();
        let source = r#"
            impl Vault {
                pub fn admin(&self, env: Env, admin: Address) {
                    for i in 0..5 {
                        log(&env, i);
                    }
                }
            }
        "#;
        let violations = rule.check(source);
        assert!(violations.is_empty());
    }

    #[test]
    fn empty_source_produces_no_findings() {
        let rule = GasExhaustionRiskRule::new();
        assert!(rule.check("").is_empty());
    }

    #[test]
    fn invalid_source_produces_no_panic() {
        let rule = GasExhaustionRiskRule::new();
        assert!(rule.check("not valid rust {{{{").is_empty());
    }
}
