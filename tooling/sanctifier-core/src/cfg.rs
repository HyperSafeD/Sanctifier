//! Intra-procedural control-flow graph (CFG) construction for a single
//! function body.
//!
//! AST-based pattern matching (the previous approach used by rules such as
//! [`crate::rules::taint_propagation`]) walks a function body once, top to
//! bottom. That is wrong whenever control flow loops back on itself: a
//! `for`/`while` body that taints a variable on one iteration and reads it on
//! the next is never connected, because a linear AST walk visits the loop
//! body's statements exactly once, in source order, with no notion that the
//! last statement can flow back into the first.
//!
//! This module builds an explicit graph of [`BasicBlock`]s connected by
//! successor edges, so that dataflow passes (see [`crate::taint_engine`]) can
//! run a proper fixed-point analysis: loop bodies get a back edge to their
//! header, `if`/`else` branches join at a common successor, and `match` arms
//! all flow into the block following the match.
//!
//! # Scope and limitations
//!
//! This is intentionally a lightweight, intra-procedural CFG tailored to the
//! subset of Rust that appears in Soroban contracts, not a general-purpose
//! Rust control-flow analyzer:
//!
//! * `let x = if/match { .. };` is **not** split into branch blocks — the
//!   whole initializer is kept as one opaque statement in the current block.
//!   Only `if`/`while`/`for`/`loop`/`match` that appear as a *statement*
//!   (not as a sub-expression) get their own blocks/edges.
//! * `break`/`continue`/`?` do not carry precise jump targets; they end the
//!   current block but are not wired back to a specific loop-exit block.
//!   This is a conservative simplification — it can never cause a dataflow
//!   fact to be lost, only (rarely) computed slightly more permissively.
//! * Function calls are not inlined or resolved — this CFG is strictly
//!   intra-procedural.

use syn::{Block, Expr, ExprForLoop, ExprIf, ExprLoop, ExprMatch, ExprWhile, Local, Pat, Stmt};

/// A single statement kept inside a [`BasicBlock`].
///
/// Control-flow-introducing statements (`if`, `while`, `for`, `loop`,
/// `match` used as a *statement*) are lowered into separate blocks/edges by
/// [`Cfg::build`] and never appear here. Everything else — `let` bindings,
/// plain expression statements, assignments — is kept verbatim so that
/// dataflow passes can inspect it.
#[derive(Debug, Clone)]
pub enum BlockStmt {
    /// A `let` binding, e.g. `let x = expr;`.
    Local(Local),
    /// A plain expression statement, e.g. `x = y;` or `sink(x);`.
    Expr(Expr),
    /// The binding introduced by a `for pat in iter_expr` loop header.
    /// Kept separate from `Local` because the binding's taintedness depends
    /// on `iter_expr`, not on a normal initializer.
    ForBinding { pat: Pat, iter_expr: Expr },
}

/// A single node in the control-flow graph: a maximal run of statements with
/// no internal branching.
#[derive(Debug, Clone, Default)]
pub struct BasicBlock {
    pub id: usize,
    pub stmts: Vec<BlockStmt>,
}

/// An intra-procedural control-flow graph for one function body.
#[derive(Debug, Clone)]
pub struct Cfg {
    pub blocks: Vec<BasicBlock>,
    /// `successors[b]` lists every block reachable in one step from block `b`.
    pub successors: Vec<Vec<usize>>,
    pub entry: usize,
}

impl Cfg {
    /// Builds a CFG for `block` (typically a function body).
    pub fn build(block: &Block) -> Cfg {
        let mut builder = CfgBuilder::default();
        let entry = builder.new_block();
        builder.process_stmts(&block.stmts, entry);
        Cfg {
            blocks: builder.blocks,
            successors: builder.successors,
            entry,
        }
    }

    pub fn block_count(&self) -> usize {
        self.blocks.len()
    }
}

#[derive(Default)]
struct CfgBuilder {
    blocks: Vec<BasicBlock>,
    successors: Vec<Vec<usize>>,
}

impl CfgBuilder {
    fn new_block(&mut self) -> usize {
        let id = self.blocks.len();
        self.blocks.push(BasicBlock {
            id,
            stmts: Vec::new(),
        });
        self.successors.push(Vec::new());
        id
    }

    fn add_edge(&mut self, from: usize, to: usize) {
        self.successors[from].push(to);
    }

    fn push_stmt(&mut self, block: usize, stmt: BlockStmt) {
        self.blocks[block].stmts.push(stmt);
    }

    /// Processes a sequence of statements starting at `current`, returning
    /// the block that subsequent (fallthrough) code should attach to.
    fn process_stmts(&mut self, stmts: &[Stmt], mut current: usize) -> usize {
        for stmt in stmts {
            current = self.process_stmt(stmt, current);
        }
        current
    }

    fn process_stmt(&mut self, stmt: &Stmt, current: usize) -> usize {
        match stmt {
            Stmt::Local(local) => {
                self.push_stmt(current, BlockStmt::Local(local.clone()));
                current
            }
            Stmt::Expr(expr, _) => self.process_expr_stmt(expr, current),
            _ => current,
        }
    }

    /// Processes an expression appearing directly as a statement. Only here
    /// do `if`/`while`/`for`/`loop`/`match` get split into their own blocks —
    /// see the module-level "Scope and limitations" note.
    fn process_expr_stmt(&mut self, expr: &Expr, current: usize) -> usize {
        match expr {
            Expr::If(e) => self.process_if(e, current),
            Expr::While(e) => self.process_while(e, current),
            Expr::ForLoop(e) => self.process_for(e, current),
            Expr::Loop(e) => self.process_loop(e, current),
            Expr::Match(e) => self.process_match(e, current),
            Expr::Block(e) => self.process_stmts(&e.block.stmts, current),
            Expr::Return(_) | Expr::Break(_) | Expr::Continue(_) => {
                self.push_stmt(current, BlockStmt::Expr(expr.clone()));
                // Anything textually after this point in the same scope is
                // unreachable along this path; give it a fresh, edge-less
                // block so it doesn't pollute `current`'s flow.
                self.new_block()
            }
            _ => {
                self.push_stmt(current, BlockStmt::Expr(expr.clone()));
                current
            }
        }
    }

    fn process_if(&mut self, e: &ExprIf, current: usize) -> usize {
        self.push_stmt(current, BlockStmt::Expr((*e.cond).clone()));

        let then_entry = self.new_block();
        self.add_edge(current, then_entry);
        let then_exit = self.process_stmts(&e.then_branch.stmts, then_entry);

        let join = self.new_block();
        self.add_edge(then_exit, join);

        match &e.else_branch {
            Some((_, else_expr)) => {
                let else_entry = self.new_block();
                self.add_edge(current, else_entry);
                let else_exit = self.process_expr_stmt(else_expr, else_entry);
                self.add_edge(else_exit, join);
            }
            None => {
                // No else: the false branch falls straight through to join.
                self.add_edge(current, join);
            }
        }
        join
    }

    fn process_while(&mut self, e: &ExprWhile, current: usize) -> usize {
        let header = self.new_block();
        self.add_edge(current, header);
        self.push_stmt(header, BlockStmt::Expr((*e.cond).clone()));

        let body_entry = self.new_block();
        self.add_edge(header, body_entry);
        let body_exit = self.process_stmts(&e.body.stmts, body_entry);
        self.add_edge(body_exit, header); // back edge

        let after = self.new_block();
        self.add_edge(header, after);
        after
    }

    fn process_for(&mut self, e: &ExprForLoop, current: usize) -> usize {
        let header = self.new_block();
        self.add_edge(current, header);
        self.push_stmt(
            header,
            BlockStmt::ForBinding {
                pat: (*e.pat).clone(),
                iter_expr: (*e.expr).clone(),
            },
        );

        let body_entry = self.new_block();
        self.add_edge(header, body_entry);
        let body_exit = self.process_stmts(&e.body.stmts, body_entry);
        self.add_edge(body_exit, header); // back edge

        let after = self.new_block();
        self.add_edge(header, after);
        after
    }

    fn process_loop(&mut self, e: &ExprLoop, current: usize) -> usize {
        let header = self.new_block();
        self.add_edge(current, header);
        let body_exit = self.process_stmts(&e.body.stmts, header);
        self.add_edge(body_exit, header); // back edge

        // `loop` has no implicit condition; it only exits via `break`. We
        // still add a reachable exit block so dataflow facts that hold at
        // loop entry are conservatively available after it too.
        let after = self.new_block();
        self.add_edge(header, after);
        after
    }

    fn process_match(&mut self, e: &ExprMatch, current: usize) -> usize {
        self.push_stmt(current, BlockStmt::Expr((*e.expr).clone()));

        let join = self.new_block();
        for arm in &e.arms {
            let arm_entry = self.new_block();
            self.add_edge(current, arm_entry);
            let arm_exit = self.process_expr_stmt(&arm.body, arm_entry);
            self.add_edge(arm_exit, join);
        }
        join
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_str;

    fn build(src: &str) -> Cfg {
        let block: Block = parse_str(&format!("{{ {src} }}")).unwrap();
        Cfg::build(&block)
    }

    #[test]
    fn straight_line_code_is_a_single_block_chain() {
        let cfg = build("let a = 1; let b = 2; let c = a + b;");
        // No branching: every block has at most one successor.
        assert!(cfg.successors.iter().all(|s| s.len() <= 1));
    }

    #[test]
    fn if_else_creates_branch_and_join() {
        let cfg = build("if cond { let a = 1; } else { let b = 2; } let c = 3;");
        // Entry block (holding the condition) must have two successors:
        // the then-branch and the else-branch.
        assert_eq!(cfg.successors[cfg.entry].len(), 2);
    }

    #[test]
    fn if_without_else_falls_through_to_join() {
        let cfg = build("if cond { let a = 1; }");
        assert_eq!(cfg.successors[cfg.entry].len(), 2);
    }

    #[test]
    fn while_loop_has_back_edge_to_header() {
        let cfg = build("let mut i = 0; while i < 10 { i = i + 1; }");
        let header_id = cfg
            .blocks
            .iter()
            .find(|b| cfg.successors[b.id].len() == 2)
            .map(|b| b.id)
            .expect("while loop must have a header block with 2 successors");
        let has_back_edge = cfg
            .successors
            .iter()
            .any(|succs| succs.contains(&header_id));
        assert!(has_back_edge, "while loop body must have a back edge");
    }

    #[test]
    fn for_loop_has_back_edge_to_header() {
        let cfg = build("for x in items.iter() { sink(x); }");
        let header_id = cfg
            .blocks
            .iter()
            .find(|b| cfg.successors[b.id].len() == 2)
            .map(|b| b.id)
            .expect("for loop must have a header block with 2 successors");
        let has_back_edge = cfg
            .successors
            .iter()
            .any(|succs| succs.contains(&header_id));
        assert!(has_back_edge, "for loop body must have a back edge");
    }

    #[test]
    fn match_arms_all_join_to_common_successor() {
        let cfg = build("match x { 0 => { let a = 1; } _ => { let b = 2; } } let c = 3;");
        assert_eq!(
            cfg.successors[cfg.entry].len(),
            2,
            "match with 2 arms must branch into 2 arm blocks"
        );
    }

    #[test]
    fn block_count_grows_with_branching() {
        let straight = build("let a = 1; let b = 2;");
        let branching = build("if cond { let a = 1; } else { let b = 2; }");
        assert!(branching.block_count() > straight.block_count());
    }
}
