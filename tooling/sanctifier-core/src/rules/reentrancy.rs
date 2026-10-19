//! Reentrancy detector for Soroban smart contracts.
//!
//! Detects the classic Soroban reentrancy pattern: a storage write (or token
//! transfer) that occurs **before** an `invoke_contract` / `invoke_contract_check`
//! call, without a boolean lock guard surrounding the external call.
//!
//! Also provides [`ReentrancyRule::fix`] which emits a [`Patch`] that inserts
//! a boolean instance-storage lock guard around the external call site.

use super::{Patch, Rule, RuleViolation, Severity};
use syn::spanned::Spanned;
use syn::{parse_str, File, Item};

/// Storage key used by the auto-generated reentrancy guard.
const REENTRANCY_LOCK_KEY: &str = "REENTRANCY_LOCK";

// ── Rule struct ───────────────────────────────────────────────────────────────

/// Rule that detects state mutations preceding external contract calls without
/// a reentrancy guard.
pub struct ReentrancyRule;

impl ReentrancyRule {
    /// Create a new instance.
    pub fn new() -> Self {
        Self
    }
}

impl Default for ReentrancyRule {
    fn default() -> Self {
        Self::new()
    }
}

// ── Internal analysis types ───────────────────────────────────────────────────

/// Tracks what we have seen so far while walking a function body.
#[derive(Default)]
struct FnReentrancyState {
    /// True once we have seen a storage write or token transfer.
    has_prior_mutation: bool,
    /// True if the function already has a reentrancy guard (lock/unlock pattern).
    has_guard: bool,
    /// Collected violations.
    violations: Vec<ReentrancyViolation>,
}

struct ReentrancyViolation {
    fn_name: String,
    line: usize,
}

// ── Rule implementation ───────────────────────────────────────────────────────

impl Rule for ReentrancyRule {
    fn name(&self) -> &str {
        "reentrancy"
    }

    fn description(&self) -> &str {
        "Detects state mutations (storage writes / token transfers) that precede \
         an invoke_contract call without a reentrancy guard"
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
                        let mut state = FnReentrancyState::default();
                        scan_block(&f.block, &fn_name, &mut state);

                        for v in state.violations {
                            violations.push(
                                RuleViolation::new(
                                    self.name(),
                                    Severity::Error,
                                    format!(
                                        "Function '{}' writes to storage before calling \
                                         invoke_contract at line {} without a reentrancy guard",
                                        v.fn_name, v.line
                                    ),
                                    format!("{}:{}", v.fn_name, v.line),
                                )
                                .with_suggestion(
                                    "Wrap the external call with a boolean instance-storage \
                                     lock: set REENTRANCY_LOCK to true before the call and \
                                     false after, panicking if already true."
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

    /// Emit a [`Patch`] that inserts a boolean lock guard around the first
    /// `invoke_contract` call in each vulnerable function.
    ///
    /// The generated guard follows the pattern described in issue #279:
    ///
    /// ```text
    /// let guarded = env.storage().instance().get::<_, bool>(&REENTRANCY_LOCK).unwrap_or(false);
    /// if guarded { panic!("reentrant call"); }
    /// env.storage().instance().set(&REENTRANCY_LOCK, &true);
    /// let result = env.invoke_contract(...);
    /// env.storage().instance().set(&REENTRANCY_LOCK, &false);
    /// ```
    fn fix(&self, source: &str) -> Vec<Patch> {
        let file = match parse_str::<File>(source) {
            Ok(f) => f,
            Err(_) => return vec![],
        };

        let mut patches = Vec::new();

        for item in &file.items {
            if let Item::Impl(impl_block) = item {
                for impl_item in &impl_block.items {
                    if let syn::ImplItem::Fn(f) = impl_item {
                        let fn_name = f.sig.ident.to_string();
                        let mut state = FnReentrancyState::default();
                        scan_block(&f.block, &fn_name, &mut state);

                        if state.violations.is_empty() {
                            continue;
                        }

                        // Only patch when there is exactly one external call (single
                        // external-call pattern as required by the acceptance criteria).
                        let call_count = count_invoke_contract_calls(&f.block);
                        if call_count != 1 {
                            continue;
                        }

                        if let Some(patch) = build_guard_patch(&f.block, &fn_name) {
                            patches.push(patch);
                        }
                    }
                }
            }
        }

        patches
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

// ── AST helpers ───────────────────────────────────────────────────────────────

/// Walk a block, updating `state` as we encounter mutations and external calls.
fn scan_block(block: &syn::Block, fn_name: &str, state: &mut FnReentrancyState) {
    for stmt in &block.stmts {
        match stmt {
            syn::Stmt::Local(local) => {
                if let Some(init) = &local.init {
                    scan_expr(&init.expr, fn_name, state);
                }
            }
            syn::Stmt::Expr(expr, _) => scan_expr(expr, fn_name, state),
            syn::Stmt::Macro(m) if m.mac.path.is_ident("panic") => {
                // detect panic!("reentrant call") as a guard indicator
                let tokens = m.mac.tokens.to_string();
                if tokens.contains("reentrant") || tokens.contains("reentrancy") {
                    state.has_guard = true;
                }
            }
            _ => {}
        }
    }
}

fn scan_expr(expr: &syn::Expr, fn_name: &str, state: &mut FnReentrancyState) {
    match expr {
        syn::Expr::MethodCall(mc) => {
            let method = mc.method.to_string();

            // Detect guard lock/unlock: storage().instance().set(&REENTRANCY_LOCK, ...)
            if method == "set" {
                let receiver_str = quote::quote!(#mc).to_string();
                if receiver_str.contains("REENTRANCY_LOCK")
                    || receiver_str.contains("reentrancy_lock")
                    || receiver_str.contains("RE_GRD")
                {
                    state.has_guard = true;
                }
            }

            // Detect storage mutations
            if is_storage_mutation(&method, &mc.receiver) {
                state.has_prior_mutation = true;
            }

            // Detect token transfers (transfer / transfer_from / burn)
            if is_token_transfer(&method) {
                state.has_prior_mutation = true;
            }

            // Detect invoke_contract / invoke_contract_check
            if (method == "invoke_contract" || method == "invoke_contract_check")
                && state.has_prior_mutation
                && !state.has_guard
            {
                let span = mc.span();
                state.violations.push(ReentrancyViolation {
                    fn_name: fn_name.to_string(),
                    line: span.start().line,
                });
            }

            // Recurse
            scan_expr(&mc.receiver, fn_name, state);
            for arg in &mc.args {
                scan_expr(arg, fn_name, state);
            }
        }
        syn::Expr::Call(c) => {
            if let syn::Expr::Path(p) = &*c.func {
                if let Some(seg) = p.path.segments.last() {
                    let ident = seg.ident.to_string();
                    if (ident == "invoke_contract" || ident == "invoke_contract_check")
                        && state.has_prior_mutation
                        && !state.has_guard
                    {
                        let span = c.span();
                        state.violations.push(ReentrancyViolation {
                            fn_name: fn_name.to_string(),
                            line: span.start().line,
                        });
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
            scan_block(&i.then_branch, fn_name, state);
            if let Some((_, else_expr)) = &i.else_branch {
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

fn is_storage_mutation(method: &str, receiver: &syn::Expr) -> bool {
    // Soroban v21 adds extend_ttl as a storage-modifying operation
    if !matches!(method, "set" | "update" | "remove" | "extend_ttl") {
        return false;
    }
    let receiver_str = quote::quote!(#receiver).to_string();
    receiver_str.contains("storage")
        || receiver_str.contains("persistent")
        || receiver_str.contains("temporary")
        || receiver_str.contains("instance")
}

fn is_token_transfer(method: &str) -> bool {
    matches!(method, "transfer" | "transfer_from" | "burn" | "burn_from")
}

fn count_invoke_contract_calls(block: &syn::Block) -> usize {
    let mut count = 0;
    count_in_block(block, &mut count);
    count
}

fn count_in_block(block: &syn::Block, count: &mut usize) {
    for stmt in &block.stmts {
        match stmt {
            syn::Stmt::Local(l) => {
                if let Some(init) = &l.init {
                    count_in_expr(&init.expr, count);
                }
            }
            syn::Stmt::Expr(e, _) => count_in_expr(e, count),
            _ => {}
        }
    }
}

fn count_in_expr(expr: &syn::Expr, count: &mut usize) {
    match expr {
        syn::Expr::MethodCall(mc) => {
            let method = mc.method.to_string();
            if method == "invoke_contract" || method == "invoke_contract_check" {
                *count += 1;
            }
            count_in_expr(&mc.receiver, count);
            for arg in &mc.args {
                count_in_expr(arg, count);
            }
        }
        syn::Expr::Call(c) => {
            if let syn::Expr::Path(p) = &*c.func {
                if let Some(seg) = p.path.segments.last() {
                    let ident = seg.ident.to_string();
                    if ident == "invoke_contract" || ident == "invoke_contract_check" {
                        *count += 1;
                    }
                }
            }
            for arg in &c.args {
                count_in_expr(arg, count);
            }
        }
        syn::Expr::Block(b) => count_in_block(&b.block, count),
        syn::Expr::If(i) => {
            count_in_expr(&i.cond, count);
            count_in_block(&i.then_branch, count);
            if let Some((_, e)) = &i.else_branch {
                count_in_expr(e, count);
            }
        }
        syn::Expr::Match(m) => {
            count_in_expr(&m.expr, count);
            for arm in &m.arms {
                count_in_expr(&arm.body, count);
            }
        }
        _ => {}
    }
}

/// Build a patch that inserts the lock/unlock guard around the `invoke_contract`
/// call site.  The patch replaces the statement containing the call with the
/// guarded version.
fn build_guard_patch(block: &syn::Block, fn_name: &str) -> Option<Patch> {
    for stmt in &block.stmts {
        let (expr, span) = match stmt {
            syn::Stmt::Local(l) => {
                if let Some(init) = &l.init {
                    let s = l.span();
                    (&*init.expr, s)
                } else {
                    continue;
                }
            }
            syn::Stmt::Expr(e, _) => (e, e.span()),
            _ => continue,
        };

        if contains_invoke_contract(expr) {
            let original = quote::quote!(#stmt).to_string();
            let replacement = format!(
                "let __guarded = env.storage().instance().get::<_, bool>(&{lock}).unwrap_or(false);\n\
                 if __guarded {{ panic!(\"reentrant call\"); }}\n\
                 env.storage().instance().set(&{lock}, &true);\n\
                 {original}\n\
                 env.storage().instance().set(&{lock}, &false);",
                lock = REENTRANCY_LOCK_KEY,
                original = original,
            );

            return Some(Patch {
                start_line: span.start().line,
                start_column: span.start().column,
                end_line: span.end().line,
                end_column: span.end().column,
                replacement,
                description: format!(
                    "Insert reentrancy guard around invoke_contract in '{}'",
                    fn_name
                ),
            });
        }
    }
    None
}

fn contains_invoke_contract(expr: &syn::Expr) -> bool {
    match expr {
        syn::Expr::MethodCall(mc) => {
            let method = mc.method.to_string();
            if method == "invoke_contract" || method == "invoke_contract_check" {
                return true;
            }
            contains_invoke_contract(&mc.receiver) || mc.args.iter().any(contains_invoke_contract)
        }
        syn::Expr::Call(c) => {
            if let syn::Expr::Path(p) = &*c.func {
                if let Some(seg) = p.path.segments.last() {
                    let ident = seg.ident.to_string();
                    if ident == "invoke_contract" || ident == "invoke_contract_check" {
                        return true;
                    }
                }
            }
            c.args.iter().any(contains_invoke_contract)
        }
        syn::Expr::Block(b) => b.block.stmts.iter().any(|s| match s {
            syn::Stmt::Expr(e, _) => contains_invoke_contract(e),
            syn::Stmt::Local(l) => l
                .init
                .as_ref()
                .map(|i| contains_invoke_contract(&i.expr))
                .unwrap_or(false),
            _ => false,
        }),
        _ => false,
    }
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn rule() -> ReentrancyRule {
        ReentrancyRule::new()
    }

    // ── True-positive cases ───────────────────────────────────────────────────

    #[test]
    fn detects_storage_write_before_invoke_contract() {
        let source = r#"
            impl MyContract {
                pub fn withdraw(env: Env, amount: i128) {
                    env.storage().persistent().set(&symbol_short!("BAL"), &0i128);
                    let result = env.invoke_contract(&other_id, &symbol_short!("recv"), vec![]);
                }
            }
        "#;
        let violations = rule().check(source);
        assert!(!violations.is_empty(), "should detect reentrancy");
        assert!(violations[0].message.contains("withdraw"));
        assert_eq!(violations[0].severity, Severity::Error);
    }

    #[test]
    fn detects_token_transfer_before_invoke_contract() {
        let source = r#"
            impl MyContract {
                pub fn pay_and_call(env: Env) {
                    token_client.transfer(&env, &from, &to, &amount);
                    env.invoke_contract(&other, &sym, vec![]);
                }
            }
        "#;
        let violations = rule().check(source);
        assert!(
            !violations.is_empty(),
            "token transfer before invoke should be flagged"
        );
    }

    #[test]
    fn detects_storage_remove_before_invoke_contract() {
        let source = r#"
            impl MyContract {
                pub fn delete_and_call(env: Env) {
                    env.storage().instance().remove(&symbol_short!("KEY"));
                    env.invoke_contract(&other, &sym, vec![]);
                }
            }
        "#;
        let violations = rule().check(source);
        assert!(
            !violations.is_empty(),
            "storage remove before invoke should be flagged"
        );
    }

    // ── False-positive / safe cases ───────────────────────────────────────────

    #[test]
    fn no_violation_when_invoke_before_storage_write() {
        let source = r#"
            impl MyContract {
                pub fn safe_call(env: Env) {
                    let result = env.invoke_contract(&other, &sym, vec![]);
                    env.storage().persistent().set(&symbol_short!("BAL"), &result);
                }
            }
        "#;
        let violations = rule().check(source);
        assert!(
            violations.is_empty(),
            "invoke before write is safe (checks-effects-interactions)"
        );
    }

    #[test]
    fn no_violation_when_reentrancy_guard_present() {
        let source = r#"
            impl MyContract {
                pub fn guarded_withdraw(env: Env) {
                    env.storage().persistent().set(&symbol_short!("BAL"), &0i128);
                    let guarded = env.storage().instance().get::<_, bool>(&REENTRANCY_LOCK).unwrap_or(false);
                    if guarded { panic!("reentrant call"); }
                    env.storage().instance().set(&REENTRANCY_LOCK, &true);
                    let result = env.invoke_contract(&other, &sym, vec![]);
                    env.storage().instance().set(&REENTRANCY_LOCK, &false);
                }
            }
        "#;
        let violations = rule().check(source);
        assert!(
            violations.is_empty(),
            "guarded function must not be flagged"
        );
    }

    #[test]
    fn no_violation_for_read_only_function() {
        let source = r#"
            impl MyContract {
                pub fn query(env: Env) -> i128 {
                    env.storage().persistent().get(&symbol_short!("BAL")).unwrap_or(0)
                }
            }
        "#;
        let violations = rule().check(source);
        assert!(
            violations.is_empty(),
            "read-only function must not be flagged"
        );
    }

    #[test]
    fn no_violation_on_empty_source() {
        let violations = rule().check("");
        assert!(violations.is_empty());
    }

    // ── reentrancy-guard contract produces zero false positives ───────────────

    #[test]
    fn zero_false_positives_against_reentrancy_guard_contract() {
        // Simplified version of contracts/reentrancy-guard/src/lib.rs
        let source = r#"
            impl ReentrancyGuard {
                pub fn enter(env: Env) {
                    let status: u32 = env.storage().instance().get(&GUARD_KEY).unwrap_or(0);
                    if status != 0 { panic!("reentrancy detected"); }
                    env.storage().instance().set(&GUARD_KEY, &1u32);
                }
                pub fn exit(env: Env) {
                    env.storage().instance().set(&GUARD_KEY, &0u32);
                }
            }
        "#;
        let violations = rule().check(source);
        assert!(
            violations.is_empty(),
            "reentrancy-guard contract must produce zero false positives, got: {:?}",
            violations
        );
    }

    // ── fix() tests ───────────────────────────────────────────────────────────

    #[test]
    fn fix_inserts_guard_for_single_invoke_contract() {
        let source = r#"
            impl MyContract {
                pub fn withdraw(env: Env) {
                    env.storage().persistent().set(&symbol_short!("BAL"), &0i128);
                    let result = env.invoke_contract(&other, &sym, vec![]);
                }
            }
        "#;
        let patches = rule().fix(source);
        assert!(!patches.is_empty(), "fix should produce a patch");
        let patch = &patches[0];
        assert!(patch.replacement.contains("REENTRANCY_LOCK"));
        assert!(patch.replacement.contains("panic!"));
        assert!(patch.replacement.contains("invoke_contract"));
    }

    #[test]
    fn fix_does_not_patch_already_guarded_function() {
        let source = r#"
            impl MyContract {
                pub fn guarded(env: Env) {
                    env.storage().persistent().set(&symbol_short!("BAL"), &0i128);
                    let guarded = env.storage().instance().get::<_, bool>(&REENTRANCY_LOCK).unwrap_or(false);
                    if guarded { panic!("reentrant call"); }
                    env.storage().instance().set(&REENTRANCY_LOCK, &true);
                    let result = env.invoke_contract(&other, &sym, vec![]);
                    env.storage().instance().set(&REENTRANCY_LOCK, &false);
                }
            }
        "#;
        let patches = rule().fix(source);
        assert!(
            patches.is_empty(),
            "already-guarded function must not be patched"
        );
    }
}
