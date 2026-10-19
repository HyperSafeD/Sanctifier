//! Intra-procedural taint analysis on top of [`crate::cfg`].
//!
//! Identifies **sources** (untrusted parameters passed into a contract entry
//! point) and **sinks** (privileged operations: storage writes and external
//! contract calls), then runs a forward dataflow fixed-point over the CFG to
//! decide whether tainted data can reach a sink. Unlike a single top-to-bottom
//! AST walk, this correctly handles:
//!
//! * **Loops** — a variable tainted at the bottom of a loop body is visible
//!   at the top of the next iteration, via the CFG's back edge.
//! * **Branch joins** — a variable tainted in only one arm of an `if`/`match`
//!   is still considered (conservatively) tainted after the branches join.
//! * **Aliasing / reassignment** — `let y = x;` propagates taint from `x` to
//!   `y`; a later `x = clean_value();` clears (kills) `x`'s own taint without
//!   affecting `y`, since each variable's taint is tracked independently and
//!   blocks are processed in their own statement order.
//!
//! # Soundness note
//!
//! This is a best-effort static analysis, not a soundness-certified one. The
//! "facts" set is unioned at join points, which is conservative (taint-safe)
//! for taint itself, but also applied to the `require_auth` marker — meaning
//! a branch that authorizes is treated as authorizing the merged path too.
//! This matches the precision level of the rest of the rule suite.

use crate::cfg::{BasicBlock, BlockStmt, Cfg};
use std::collections::{HashSet, VecDeque};
use syn::spanned::Spanned;
use syn::{Expr, Pat};

/// Sentinel inserted into the fact set once a `require_auth` /
/// `require_auth_for_args` call has been observed on a path.
const AUTH_MARKER: &str = "__authorized__";

/// A tainted value reaching a sink.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaintFinding {
    /// The tainted variable observed at the sink call site.
    pub var: String,
    /// The sink method name (e.g. `"set"`, `"invoke_contract"`).
    pub sink: String,
    /// Source line of the sink call, if available.
    pub line: usize,
}

/// Runs taint analysis over `body`, seeding the analysis with `sources` (the
/// set of variable names considered tainted at function entry).
///
/// A sink call is flagged unless `AUTH_MARKER` is present in the fact set at
/// that point in the CFG (i.e. some `require_auth*` call precedes it on every
/// path the dataflow fixed point considers reachable).
pub fn analyze(body: &syn::Block, sources: HashSet<String>) -> Vec<TaintFinding> {
    let cfg = Cfg::build(body);
    let n = cfg.block_count();
    if n == 0 {
        return vec![];
    }

    let mut block_in: Vec<HashSet<String>> = vec![HashSet::new(); n];
    block_in[cfg.entry] = sources;

    let mut worklist: VecDeque<usize> = (0..n).collect();
    while let Some(b) = worklist.pop_front() {
        let (out, _) = transfer(&cfg.blocks[b], &block_in[b]);
        for &succ in &cfg.successors[b] {
            let before = block_in[succ].len();
            block_in[succ].extend(out.iter().cloned());
            if block_in[succ].len() != before {
                worklist.push_back(succ);
            }
        }
    }

    // Final pass over the stabilized fixed point to collect findings without
    // duplication from intermediate iterations.
    let mut findings = Vec::new();
    for block in &cfg.blocks {
        let (_, block_findings) = transfer(block, &block_in[block.id]);
        findings.extend(block_findings);
    }
    findings
}

/// Applies one basic block's statements to an incoming fact set, returning
/// the outgoing fact set and any sink findings observed along the way.
fn transfer(
    block: &BasicBlock,
    in_facts: &HashSet<String>,
) -> (HashSet<String>, Vec<TaintFinding>) {
    let mut facts = in_facts.clone();
    let mut findings = Vec::new();

    for stmt in &block.stmts {
        match stmt {
            BlockStmt::Local(local) => {
                if let Some(init) = &local.init {
                    scan_expr(&init.expr, &mut facts, &mut findings);
                    if expr_is_tainted(&init.expr, &facts) {
                        let mut names = HashSet::new();
                        collect_pat_idents(&local.pat, &mut names);
                        facts.extend(names);
                    } else if let Some(name) = simple_pat_ident(&local.pat) {
                        // Strong update: rebinding to a clean value clears
                        // this variable's own taint.
                        facts.remove(&name);
                    }
                }
            }
            BlockStmt::Expr(expr) => process_top_level_expr(expr, &mut facts, &mut findings),
            BlockStmt::ForBinding { pat, iter_expr } => {
                scan_expr(iter_expr, &mut facts, &mut findings);
                if expr_is_tainted(iter_expr, &facts) {
                    let mut names = HashSet::new();
                    collect_pat_idents(pat, &mut names);
                    facts.extend(names);
                }
            }
        }
    }

    (facts, findings)
}

fn process_top_level_expr(
    expr: &Expr,
    facts: &mut HashSet<String>,
    findings: &mut Vec<TaintFinding>,
) {
    if let Expr::Assign(a) = expr {
        scan_expr(&a.right, facts, findings);
        let tainted = expr_is_tainted(&a.right, facts);
        if let Some(name) = simple_expr_ident(&a.left) {
            if tainted {
                facts.insert(name);
            } else {
                facts.remove(&name);
            }
        }
        return;
    }
    scan_expr(expr, facts, findings);
}

/// Recursively walks an expression, recording sink findings and updating the
/// auth marker, without itself binding any new taint (that only happens for
/// `let` bindings and plain assignments — see [`transfer`]).
fn scan_expr(expr: &Expr, facts: &mut HashSet<String>, findings: &mut Vec<TaintFinding>) {
    match expr {
        Expr::MethodCall(mc) => {
            let method = mc.method.to_string();

            if method == "require_auth" || method == "require_auth_for_args" {
                facts.insert(AUTH_MARKER.to_string());
            }

            if (is_storage_write(&method, &mc.receiver) || is_external_call(&method))
                && !facts.contains(AUTH_MARKER)
            {
                for arg in &mc.args {
                    if let Some(var) = first_tainted_ident(arg, facts) {
                        findings.push(TaintFinding {
                            var,
                            sink: method.clone(),
                            line: mc.span().start().line,
                        });
                    }
                }
            }

            scan_expr(&mc.receiver, facts, findings);
            for arg in &mc.args {
                scan_expr(arg, facts, findings);
            }
        }
        Expr::Call(c) => {
            scan_expr(&c.func, facts, findings);
            for arg in &c.args {
                scan_expr(arg, facts, findings);
            }
        }
        Expr::If(i) => {
            scan_expr(&i.cond, facts, findings);
            for stmt in &i.then_branch.stmts {
                scan_stmt(stmt, facts, findings);
            }
            if let Some((_, else_expr)) = &i.else_branch {
                scan_expr(else_expr, facts, findings);
            }
        }
        Expr::Match(m) => {
            scan_expr(&m.expr, facts, findings);
            for arm in &m.arms {
                scan_expr(&arm.body, facts, findings);
            }
        }
        Expr::Block(b) => {
            for stmt in &b.block.stmts {
                scan_stmt(stmt, facts, findings);
            }
        }
        Expr::Assign(a) => {
            scan_expr(&a.left, facts, findings);
            scan_expr(&a.right, facts, findings);
        }
        Expr::Paren(p) => scan_expr(&p.expr, facts, findings),
        Expr::Group(g) => scan_expr(&g.expr, facts, findings),
        Expr::Reference(r) => scan_expr(&r.expr, facts, findings),
        Expr::Unary(u) => scan_expr(&u.expr, facts, findings),
        Expr::Cast(c) => scan_expr(&c.expr, facts, findings),
        Expr::Try(t) => scan_expr(&t.expr, facts, findings),
        Expr::Field(f) => scan_expr(&f.base, facts, findings),
        Expr::Index(idx) => {
            scan_expr(&idx.expr, facts, findings);
            scan_expr(&idx.index, facts, findings);
        }
        Expr::Binary(b) => {
            scan_expr(&b.left, facts, findings);
            scan_expr(&b.right, facts, findings);
        }
        Expr::Tuple(t) => {
            for elem in &t.elems {
                scan_expr(elem, facts, findings);
            }
        }
        Expr::Return(r) => {
            if let Some(inner) = &r.expr {
                scan_expr(inner, facts, findings);
            }
        }
        _ => {}
    }
}

fn scan_stmt(stmt: &syn::Stmt, facts: &mut HashSet<String>, findings: &mut Vec<TaintFinding>) {
    match stmt {
        syn::Stmt::Local(local) => {
            if let Some(init) = &local.init {
                scan_expr(&init.expr, facts, findings);
                if expr_is_tainted(&init.expr, facts) {
                    let mut names = HashSet::new();
                    collect_pat_idents(&local.pat, &mut names);
                    facts.extend(names);
                }
            }
        }
        syn::Stmt::Expr(expr, _) => process_top_level_expr(expr, facts, findings),
        _ => {}
    }
}

// ── Pattern / expression helpers ─────────────────────────────────────────────

/// Recursively collects all identifier names bound by a pattern (handles
/// tuple/struct/tuple-struct/reference destructures).
pub fn collect_pat_idents(pat: &Pat, out: &mut HashSet<String>) {
    match pat {
        Pat::Ident(pi) => {
            out.insert(pi.ident.to_string());
        }
        Pat::Tuple(pt) => {
            for elem in &pt.elems {
                collect_pat_idents(elem, out);
            }
        }
        Pat::Struct(ps) => {
            for field in &ps.fields {
                collect_pat_idents(&field.pat, out);
            }
        }
        Pat::TupleStruct(pts) => {
            for elem in &pts.elems {
                collect_pat_idents(elem, out);
            }
        }
        Pat::Reference(pr) => collect_pat_idents(&pr.pat, out),
        Pat::Type(pt) => collect_pat_idents(&pt.pat, out),
        _ => {}
    }
}

fn simple_pat_ident(pat: &Pat) -> Option<String> {
    match pat {
        Pat::Ident(pi) => Some(pi.ident.to_string()),
        Pat::Type(pt) => simple_pat_ident(&pt.pat),
        _ => None,
    }
}

fn simple_expr_ident(expr: &Expr) -> Option<String> {
    match expr {
        Expr::Path(p) => p.path.segments.last().map(|s| s.ident.to_string()),
        _ => None,
    }
}

/// Returns `true` if `expr` references any currently-tainted variable.
pub fn expr_is_tainted(expr: &Expr, facts: &HashSet<String>) -> bool {
    first_tainted_ident(expr, facts).is_some()
}

/// Returns the first tainted identifier referenced by `expr`, if any.
fn first_tainted_ident(expr: &Expr, facts: &HashSet<String>) -> Option<String> {
    match expr {
        Expr::Path(p) => {
            let name = p.path.segments.last()?.ident.to_string();
            facts.contains(&name).then_some(name)
        }
        Expr::Reference(r) => first_tainted_ident(&r.expr, facts),
        Expr::Paren(p) => first_tainted_ident(&p.expr, facts),
        Expr::Group(g) => first_tainted_ident(&g.expr, facts),
        Expr::Unary(u) => first_tainted_ident(&u.expr, facts),
        Expr::Cast(c) => first_tainted_ident(&c.expr, facts),
        Expr::Field(f) => first_tainted_ident(&f.base, facts),
        Expr::MethodCall(mc) => first_tainted_ident(&mc.receiver, facts)
            .or_else(|| mc.args.iter().find_map(|a| first_tainted_ident(a, facts))),
        Expr::Call(c) => c.args.iter().find_map(|a| first_tainted_ident(a, facts)),
        Expr::Tuple(t) => t.elems.iter().find_map(|e| first_tainted_ident(e, facts)),
        Expr::Binary(b) => {
            first_tainted_ident(&b.left, facts).or_else(|| first_tainted_ident(&b.right, facts))
        }
        _ => None,
    }
}

fn is_storage_write(method: &str, receiver: &Expr) -> bool {
    if !matches!(method, "set" | "update" | "remove") {
        return false;
    }
    let s = quote::quote!(#receiver).to_string();
    s.contains("storage")
        || s.contains("persistent")
        || s.contains("temporary")
        || s.contains("instance")
}

fn is_external_call(method: &str) -> bool {
    matches!(
        method,
        "invoke_contract" | "try_invoke_contract" | "invoke_contract_check"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_str;

    fn analyze_src(src: &str, sources: &[&str]) -> Vec<TaintFinding> {
        let block: syn::Block = parse_str(&format!("{{ {src} }}")).unwrap();
        analyze(&block, sources.iter().map(|s| s.to_string()).collect())
    }

    #[test]
    fn flags_direct_taint_to_sink() {
        let findings = analyze_src(
            "env.storage().persistent().set(&key, &val);",
            &["key", "val"],
        );
        assert!(!findings.is_empty());
        assert_eq!(findings[0].sink, "set");
    }

    #[test]
    fn require_auth_suppresses_finding() {
        let findings = analyze_src(
            "caller.require_auth(); env.storage().persistent().set(&key, &val);",
            &["key", "val", "caller"],
        );
        assert!(findings.is_empty());
    }

    #[test]
    fn taint_survives_aliasing_through_intermediate_variable() {
        // key -> alias -> sink. A pure single-pass AST matcher that only
        // checks direct parameter names at the call site would miss this if
        // it didn't track the intermediate binding; the dataflow engine
        // must propagate taint through the rebinding.
        let findings = analyze_src(
            "let alias = key; env.storage().persistent().set(&alias, &val);",
            &["key", "val"],
        );
        assert!(
            !findings.is_empty(),
            "taint must propagate through an intermediate alias variable"
        );
    }

    #[test]
    fn reassignment_to_clean_value_kills_taint() {
        let findings = analyze_src(
            "let mut x = key; x = 0; env.storage().persistent().set(&x, &fixed_val);",
            &["key"],
        );
        assert!(
            findings.is_empty(),
            "reassigning x to a constant must clear its taint"
        );
    }

    #[test]
    fn taint_introduced_inside_loop_body_is_caught() {
        // The bug this module exists to fix: a pure top-to-bottom AST walk
        // over the original rule never even visited for-loop bodies, so
        // taint introduced by iterating a tainted collection was invisible.
        let findings = analyze_src(
            "let mut key = clean_key; for item in items.iter() { key = item; env.storage().persistent().set(&key, &val); }",
            &["items", "val"],
        );
        assert!(
            !findings.is_empty(),
            "taint flowing from a tainted iterator into the loop variable must be caught"
        );
    }

    #[test]
    fn taint_from_one_if_branch_is_visible_after_join() {
        let findings = analyze_src(
            "let mut key = clean_key; if cond { key = tainted_input; } env.storage().persistent().set(&key, &val);",
            &["tainted_input", "val"],
        );
        assert!(
            !findings.is_empty(),
            "taint introduced in only one branch must still be visible after the if/else joins"
        );
    }

    #[test]
    fn no_finding_when_no_source_reaches_sink() {
        let findings = analyze_src(
            "env.storage().persistent().set(&safe_key, &safe_val);",
            &["unrelated_param"],
        );
        assert!(findings.is_empty());
    }

    #[test]
    fn external_call_sink_is_detected() {
        let findings = analyze_src("env.invoke_contract(&target, &fn_name, args);", &["args"]);
        assert!(!findings.is_empty());
        assert_eq!(findings[0].sink, "invoke_contract");
    }
}
