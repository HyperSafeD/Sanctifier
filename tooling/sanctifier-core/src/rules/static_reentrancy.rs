//! Rule S027 — static reentrancy detection (complement to runtime guard).
//!
//! Detects the classic checks-effects-interactions violation at the AST level:
//! an external contract call (`invoke_contract` / `try_invoke_contract` /
//! `invoke_contract_check`) that is followed by a storage mutation in the same
//! function body, without a reentrancy guard.
//!
//! This is the **reverse** pattern of the existing `reentrancy` rule (S013),
//! which flags *mutations before calls*.  Here we flag *calls before mutations*,
//! the pattern that allows a malicious callee to re-enter with stale state.
//!
//! Each finding carries a confidence score:
//! - **High** — `invoke_contract` (panicking) directly precedes a storage write.
//! - **Medium** — `try_invoke_contract` precedes a storage write (recoverable call).
//! - **Low** — external call is separated from the write by control flow.

use super::{Rule, RuleViolation, Severity};
use syn::spanned::Spanned;
use syn::{parse_str, File, Item};

pub struct StaticReentrancyRule;

impl StaticReentrancyRule {
    pub fn new() -> Self {
        Self
    }
}

impl Default for StaticReentrancyRule {
    fn default() -> Self {
        Self::new()
    }
}

// ── Confidence level ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Confidence {
    High,
    Medium,
    Low,
}

impl Confidence {
    fn as_str(self) -> &'static str {
        match self {
            Confidence::High => "high",
            Confidence::Medium => "medium",
            Confidence::Low => "low",
        }
    }
}

// ── Internal state ─────────────────────────────────────────────────────────────

#[derive(Default)]
struct FnCallState {
    /// True once we have seen an external call.
    has_prior_external_call: bool,
    /// The kind of external call seen (for confidence scoring).
    external_call_kind: Option<ExternalCallKind>,
    /// True if a reentrancy guard is detected.
    has_guard: bool,
    violations: Vec<StaticReentrancyViolation>,
}

#[derive(Clone)]
struct StaticReentrancyViolation {
    fn_name: String,
    line: usize,
    confidence: Confidence,
    #[allow(dead_code)]
    call_kind: ExternalCallKind,
}

#[derive(Debug, Clone, Copy)]
enum ExternalCallKind {
    /// `invoke_contract` — panics on callee failure.
    Panicking,
    /// `try_invoke_contract` — returns Result.
    Recoverable,
    /// `invoke_contract_check` — with auth check.
    Checked,
}

impl ExternalCallKind {
    fn confidence(self) -> Confidence {
        match self {
            ExternalCallKind::Panicking => Confidence::High,
            ExternalCallKind::Checked => Confidence::Medium,
            ExternalCallKind::Recoverable => Confidence::Medium,
        }
    }
}

// ── Rule impl ──────────────────────────────────────────────────────────────────

impl Rule for StaticReentrancyRule {
    fn name(&self) -> &str {
        "static_reentrancy"
    }

    fn description(&self) -> &str {
        "Detects external contract calls that precede storage mutations in the same \
         function — classic checks-effects-interactions violation enabling reentrancy"
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
                        let mut state = FnCallState::default();
                        scan_block(&f.block, &fn_name, &mut state);

                        for v in &state.violations {
                            let confidence = v.confidence;
                            violations.push(
                                RuleViolation::new(
                                    self.name(),
                                    Severity::Warning,
                                    format!(
                                        "Function '{}': external call precedes storage mutation \
                                         at line {} without a reentrancy guard [confidence: {}]",
                                        v.fn_name,
                                        v.line,
                                        confidence.as_str()
                                    ),
                                    format!("{}:{}", v.fn_name, v.line),
                                )
                                .with_suggestion(
                                    "Follow the checks-effects-interactions pattern: \
                                     update all storage state BEFORE making external calls. \
                                     Or use a boolean instance-storage reentrancy lock: \
                                     set REENTRANCY_LOCK=true before the call, false after, \
                                     panicking if already locked."
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

// ── AST helpers ────────────────────────────────────────────────────────────────

fn scan_block(block: &syn::Block, fn_name: &str, state: &mut FnCallState) {
    for stmt in &block.stmts {
        match stmt {
            syn::Stmt::Local(l) => {
                if let Some(init) = &l.init {
                    scan_expr(&init.expr, fn_name, state);
                }
            }
            syn::Stmt::Expr(e, _) => scan_expr(e, fn_name, state),
            syn::Stmt::Macro(m) if m.mac.path.is_ident("panic") => {
                let tokens = m.mac.tokens.to_string();
                if tokens.contains("reentrant") || tokens.contains("reentrancy") {
                    state.has_guard = true;
                }
            }
            _ => {}
        }
    }
}

fn scan_expr(expr: &syn::Expr, fn_name: &str, state: &mut FnCallState) {
    match expr {
        syn::Expr::MethodCall(mc) => {
            let method = mc.method.to_string();

            // Detect reentrancy guard via lock key
            if method == "set" {
                let s = quote::quote!(#mc).to_string();
                if s.contains("REENTRANCY_LOCK")
                    || s.contains("reentrancy_lock")
                    || s.contains("RE_GRD")
                {
                    state.has_guard = true;
                }
            }

            // Detect external calls — record the first one seen
            if let Some(kind) = external_call_kind(&method) {
                if state.external_call_kind.is_none() {
                    state.has_prior_external_call = true;
                    state.external_call_kind = Some(kind);
                }
            }

            // Detect storage mutations that follow an external call
            if state.has_prior_external_call
                && !state.has_guard
                && is_storage_mutation(&method, &mc.receiver)
            {
                let span = mc.span();
                let call_kind = state.external_call_kind.unwrap();
                state.violations.push(StaticReentrancyViolation {
                    fn_name: fn_name.to_string(),
                    line: span.start().line,
                    confidence: call_kind.confidence(),
                    call_kind,
                });
            }

            scan_expr(&mc.receiver, fn_name, state);
            for arg in &mc.args {
                scan_expr(arg, fn_name, state);
            }
        }
        syn::Expr::Call(c) => {
            if let syn::Expr::Path(p) = &*c.func {
                if let Some(seg) = p.path.segments.last() {
                    let ident = seg.ident.to_string();
                    if let Some(kind) = external_call_kind(&ident) {
                        if state.external_call_kind.is_none() {
                            state.has_prior_external_call = true;
                            state.external_call_kind = Some(kind);
                        }
                    }
                }
            }
            for arg in &c.args {
                scan_expr(arg, fn_name, state);
            }
        }
        syn::Expr::Block(b) => scan_block(&b.block, fn_name, state),
        syn::Expr::If(i) => {
            scan_expr(&i.cond, fn_name, state);
            // Track state before entering branches; mutations in else-branch after a
            // call in the then-branch get a lower confidence score.
            let had_call = state.has_prior_external_call;
            scan_block(&i.then_branch, fn_name, state);
            if !had_call && state.has_prior_external_call {
                // Call was inside then-branch; mutations in else get Low confidence.
                // Mark any subsequent violations as Low.
                let call_before = state.violations.len();
                if let Some((_, else_expr)) = &i.else_branch {
                    scan_expr(else_expr, fn_name, state);
                }
                for v in state.violations.iter_mut().skip(call_before) {
                    v.confidence = Confidence::Low;
                }
            } else if let Some((_, else_expr)) = &i.else_branch {
                scan_expr(else_expr, fn_name, state);
            }
        }
        syn::Expr::Match(m) => {
            scan_expr(&m.expr, fn_name, state);
            for arm in &m.arms {
                scan_expr(&arm.body, fn_name, state);
            }
        }
        syn::Expr::Assign(a) => {
            scan_expr(&a.left, fn_name, state);
            scan_expr(&a.right, fn_name, state);
        }
        _ => {}
    }
}

fn external_call_kind(method: &str) -> Option<ExternalCallKind> {
    match method {
        "invoke_contract" => Some(ExternalCallKind::Panicking),
        "try_invoke_contract" => Some(ExternalCallKind::Recoverable),
        "invoke_contract_check" => Some(ExternalCallKind::Checked),
        _ => None,
    }
}

fn is_storage_mutation(method: &str, receiver: &syn::Expr) -> bool {
    if !matches!(method, "set" | "update" | "remove" | "extend_ttl") {
        return false;
    }
    let s = quote::quote!(#receiver).to_string();
    s.contains("storage")
        || s.contains("persistent")
        || s.contains("temporary")
        || s.contains("instance")
}

// ── Unit tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn rule() -> StaticReentrancyRule {
        StaticReentrancyRule::new()
    }

    // ── True-positive cases ───────────────────────────────────────────────────

    #[test]
    fn detects_invoke_contract_before_storage_write() {
        let source = r#"
            impl MyContract {
                pub fn unsafe_withdraw(env: Env, amount: i128) {
                    let result = env.invoke_contract(&other, &sym, vec![]);
                    env.storage().persistent().set(&symbol_short!("BAL"), &0i128);
                }
            }
        "#;
        let v = rule().check(source);
        assert!(
            !v.is_empty(),
            "invoke_contract before storage write must be flagged"
        );
        assert!(v[0].message.contains("unsafe_withdraw"));
        assert!(v[0].message.contains("high"));
    }

    #[test]
    fn detects_try_invoke_before_storage_write() {
        let source = r#"
            impl MyContract {
                pub fn call_then_write(env: Env) {
                    let _ = env.try_invoke_contract::<_, ()>(&other, &sym, vec![]);
                    env.storage().persistent().set(&symbol_short!("STATE"), &1u32);
                }
            }
        "#;
        let v = rule().check(source);
        assert!(
            !v.is_empty(),
            "try_invoke_contract before write must be flagged"
        );
        assert!(v[0].message.contains("medium"));
    }

    // ── False-positive / safe cases ───────────────────────────────────────────

    #[test]
    fn no_violation_when_write_before_call() {
        let source = r#"
            impl MyContract {
                pub fn safe_cei(env: Env, amount: i128) {
                    // Effects first, then interaction — correct CEI order
                    env.storage().persistent().set(&symbol_short!("BAL"), &0i128);
                    let result = env.invoke_contract(&other, &sym, vec![]);
                }
            }
        "#;
        let v = rule().check(source);
        assert!(
            v.is_empty(),
            "write-before-call is the safe CEI pattern and must not be flagged by static_reentrancy"
        );
    }

    #[test]
    fn no_violation_when_guard_present() {
        let source = r#"
            impl MyContract {
                pub fn guarded(env: Env) {
                    let g = env.storage().instance().get::<_, bool>(&REENTRANCY_LOCK).unwrap_or(false);
                    if g { panic!("reentrant call"); }
                    env.storage().instance().set(&REENTRANCY_LOCK, &true);
                    let result = env.invoke_contract(&other, &sym, vec![]);
                    env.storage().persistent().set(&symbol_short!("S"), &1u32);
                    env.storage().instance().set(&REENTRANCY_LOCK, &false);
                }
            }
        "#;
        let v = rule().check(source);
        assert!(v.is_empty(), "guarded function must not be flagged");
    }

    #[test]
    fn no_violation_for_read_only_function() {
        let source = r#"
            impl MyContract {
                pub fn query(env: Env) -> i128 {
                    let val: i128 = env.storage().persistent().get(&symbol_short!("BAL")).unwrap_or(0);
                    val
                }
            }
        "#;
        let v = rule().check(source);
        assert!(v.is_empty(), "read-only function must not be flagged");
    }

    #[test]
    fn no_violation_on_empty_source() {
        assert!(rule().check("").is_empty());
    }

    #[test]
    fn no_violation_for_external_call_only() {
        let source = r#"
            impl MyContract {
                pub fn just_call(env: Env) {
                    let result = env.invoke_contract(&other, &sym, vec![]);
                }
            }
        "#;
        let v = rule().check(source);
        assert!(
            v.is_empty(),
            "external call without subsequent write must not be flagged"
        );
    }
}
