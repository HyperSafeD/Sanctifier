// tooling/sanctifier-core/src/complexity.rs
//
// Contract Complexity Metrics — Issue #45
//
// Metrics per public function:
//   - Cyclomatic complexity (branches + 1)
//   - Parameter count
//   - Max nesting depth
//   - Lines of code (LOC)
//   - Number of extern crate / use dependencies (file-level)

use syn::{visit::Visit, File, ImplItem, ImplItemFn, ItemFn, ItemUse, ItemExternCrate};

// ---------------------------------------------------------------------------
// Thresholds (warn if exceeded)
// ---------------------------------------------------------------------------
const THRESHOLD_CYCLOMATIC: u32 = 10;
const THRESHOLD_PARAMS: usize   = 5;
const THRESHOLD_NESTING: u32    = 4;
const THRESHOLD_LOC: usize      = 50;

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize)]
pub struct FunctionMetrics {
    pub name: String,
    pub cyclomatic_complexity: u32,
    pub param_count: usize,
    pub max_nesting_depth: u32,
    pub loc: usize,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ContractMetrics {
    pub contract_path: String,
    pub dependency_count: usize,
    pub functions: Vec<FunctionMetrics>,
}

// ---------------------------------------------------------------------------
// Visitor for a single function body
// ---------------------------------------------------------------------------

struct FnComplexityVisitor {
    cyclomatic: u32,
    current_depth: u32,
    max_depth: u32,
}

impl FnComplexityVisitor {
    fn new() -> Self {
        Self { cyclomatic: 1, current_depth: 0, max_depth: 0 }
    }

    fn enter(&mut self) {
        self.current_depth += 1;
        if self.current_depth > self.max_depth {
            self.max_depth = self.current_depth;
        }
    }

    fn exit(&mut self) {
        self.current_depth = self.current_depth.saturating_sub(1);
    }
}

impl<'ast> Visit<'ast> for FnComplexityVisitor {
    fn visit_expr_if(&mut self, node: &'ast syn::ExprIf) {
        self.cyclomatic += 1;
        self.enter();
        syn::visit::visit_expr_if(self, node);
        self.exit();
    }
    fn visit_expr_match(&mut self, node: &'ast syn::ExprMatch) {
        // Each arm beyond the first adds a branch
        self.cyclomatic += node.arms.len().saturating_sub(1) as u32;
        self.enter();
        syn::visit::visit_expr_match(self, node);
        self.exit();
    }
    fn visit_expr_for_loop(&mut self, node: &'ast syn::ExprForLoop) {
        self.cyclomatic += 1;
        self.enter();
        syn::visit::visit_expr_for_loop(self, node);
        self.exit();
    }
    fn visit_expr_while(&mut self, node: &'ast syn::ExprWhile) {
        self.cyclomatic += 1;
        self.enter();
        syn::visit::visit_expr_while(self, node);
        self.exit();
    }
    fn visit_expr_loop(&mut self, node: &'ast syn::ExprLoop) {
        self.cyclomatic += 1;
        self.enter();
        syn::visit::visit_expr_loop(self, node);
        self.exit();
    }
    fn visit_expr_closure(&mut self, node: &'ast syn::ExprClosure) {
        self.cyclomatic += 1;
        self.enter();
        syn::visit::visit_expr_closure(self, node);
        self.exit();
    }
    // &&, || add logical branches
    fn visit_expr_binary(&mut self, node: &'ast syn::ExprBinary) {
        use syn::BinOp::*;
        if matches!(node.op, And(_) | Or(_)) {
            self.cyclomatic += 1;
        }
        syn::visit::visit_expr_binary(self, node);
    }
}

// ---------------------------------------------------------------------------
// File-level visitor (collects functions + dependency count)
// ---------------------------------------------------------------------------

struct FileVisitor {
    pub functions: Vec<FunctionMetrics>,
    pub dependency_count: usize,
}

impl FileVisitor {
    fn new() -> Self {
        Self { functions: Vec::new(), dependency_count: 0 }
    }

    fn analyze_fn(&self, name: &str, sig: &syn::Signature, block: &syn::Block, span_str: &str) -> FunctionMetrics {
        let mut visitor = FnComplexityVisitor::new();
        visitor.visit_block(block);

        let param_count = sig.inputs.len();
        let loc = count_loc(span_str);

        let mut warnings = Vec::new();
        if visitor.cyclomatic > THRESHOLD_CYCLOMATIC {
            warnings.push(format!(
                "Cyclomatic complexity {} exceeds threshold {}",
                visitor.cyclomatic, THRESHOLD_CYCLOMATIC
            ));
        }
        if param_count > THRESHOLD_PARAMS {
            warnings.push(format!(
                "{} parameters exceeds threshold {}",
                param_count, THRESHOLD_PARAMS
            ));
        }
        if visitor.max_depth > THRESHOLD_NESTING {
            warnings.push(format!(
                "Nesting depth {} exceeds threshold {}",
                visitor.max_depth, THRESHOLD_NESTING
            ));
        }
        if loc > THRESHOLD_LOC {
            warnings.push(format!(
                "{} LOC exceeds threshold {}",
                loc, THRESHOLD_LOC
            ));
        }

        FunctionMetrics {
            name: name.to_string(),
            cyclomatic_complexity: visitor.cyclomatic,
            param_count,
            max_nesting_depth: visitor.max_depth,
            loc,
            warnings,
        }
    }
}

impl<'ast> Visit<'ast> for FileVisitor {
    fn visit_item_use(&mut self, _: &'ast ItemUse) {
        self.dependency_count += 1;
    }
    fn visit_item_extern_crate(&mut self, _: &'ast ItemExternCrate) {
        self.dependency_count += 1;
    }

    fn visit_item_fn(&mut self, node: &'ast ItemFn) {
        // Only public functions
        if matches!(node.vis, syn::Visibility::Public(_)) {
            let span_str = quote::quote!(#node).to_string();
            let m = self.analyze_fn(&node.sig.ident.to_string(), &node.sig, &node.block, &span_str);
            self.functions.push(m);
        }
        syn::visit::visit_item_fn(self, node);
    }

    fn visit_impl_item_fn(&mut self, node: &'ast ImplItemFn) {
        let span_str = quote::quote!(#node).to_string();
        let m = self.analyze_fn(&node.sig.ident.to_string(), &node.sig, &node.block, &span_str);
        self.functions.push(m);
        syn::visit::visit_impl_item_fn(self, node);
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

pub fn analyze_complexity(ast: &File, contract_path: &str) -> ContractMetrics {
    let mut visitor = FileVisitor::new();
    visitor.visit_file(ast);

    ContractMetrics {
        contract_path: contract_path.to_string(),
        dependency_count: visitor.dependency_count,
        functions: visitor.functions,
    }
}

/// Render plain-text report
pub fn render_text_report(metrics: &ContractMetrics) -> String {
    let mut out = String::new();
    out.push_str("╔══════════════════════════════════════════════════════════════════════╗\n");
    out.push_str("║       📊  SANCTIFIER — CONTRACT COMPLEXITY REPORT                    ║\n");
    out.push_str("╚══════════════════════════════════════════════════════════════════════╝\n\n");
    out.push_str(&format!("  Contract    : {}\n", metrics.contract_path));
    out.push_str(&format!("  Dependencies: {}\n", metrics.dependency_count));
    out.push_str(&format!("  Functions   : {}\n\n", metrics.functions.len()));

    out.push_str("┌──────────────────────────┬──────┬────────┬─────────┬─────┬──────────┐\n");
    out.push_str("│ Function                 │ CC   │ Params │ Nesting │ LOC │ Status   │\n");
    out.push_str("├──────────────────────────┼──────┼────────┼─────────┼─────┼──────────┤\n");

    for f in &metrics.functions {
        let status = if f.warnings.is_empty() { "✅ OK" } else { "⚠️  WARN" };
        out.push_str(&format!(
            "│ {:<24} │ {:>4} │ {:>6} │ {:>7} │ {:>3} │ {:<8} │\n",
            truncate(&f.name, 24),
            f.cyclomatic_complexity,
            f.param_count,
            f.max_nesting_depth,
            f.loc,
            status,
        ));
    }
    out.push_str("└──────────────────────────┴──────┴────────┴─────────┴─────┴──────────┘\n\n");

    // Warnings section
    let has_warnings = metrics.functions.iter().any(|f| !f.warnings.is_empty());
    if has_warnings {
        out.push_str("  WARNINGS:\n");
        for f in &metrics.functions {
            for w in &f.warnings {
                out.push_str(&format!("  ⚠️  {}(): {}\n", f.name, w));
            }
        }
        out.push('\n');
    }

    out.push_str("  Thresholds: CC > 10 | Params > 5 | Nesting > 4 | LOC > 50\n");
    out
}

/// Render JSON report
pub fn render_json_report(metrics: &ContractMetrics) -> String {
    serde_json::to_string_pretty(metrics).unwrap_or_else(|_| "{}".to_string())
}

/// Render HTML report
pub fn render_html_report(metrics: &ContractMetrics) -> String {
    let rows: String = metrics.functions.iter().map(|f| {
        let warn_class = if f.warnings.is_empty() { "ok" } else { "warn" };
        let cc_class   = if f.cyclomatic_complexity > THRESHOLD_CYCLOMATIC { "over" } else { "" };
        let p_class    = if f.param_count > THRESHOLD_PARAMS { "over" } else { "" };
        let n_class    = if f.max_nesting_depth > THRESHOLD_NESTING { "over" } else { "" };
        let l_class    = if f.loc > THRESHOLD_LOC { "over" } else { "" };
        let warnings   = f.warnings.iter().map(|w| format!("<li>{}</li>", w)).collect::<String>();
        let warn_block = if warnings.is_empty() { String::new() }
            else { format!("<ul class='warn-list'>{}</ul>", warnings) };
        format!(
            "<tr class='{warn_class}'>\
              <td>{}</td>\
              <td class='{cc_class}'>{}</td>\
              <td class='{p_class}'>{}</td>\
              <td class='{n_class}'>{}</td>\
              <td class='{l_class}'>{}</td>\
              <td>{}</td>\
            </tr>{}\n",
            f.name,
            f.cyclomatic_complexity,
            f.param_count,
            f.max_nesting_depth,
            f.loc,
            if f.warnings.is_empty() { "✅" } else { "⚠️" },
            if warn_block.is_empty() { String::new() }
            else { format!("<tr class='warn-detail'><td colspan='6'>{}</td></tr>", warn_block) }
        )
    }).collect();

    format!(r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<title>Sanctifier — Complexity Report</title>
<style>
  body {{ font-family: monospace; background: #0d1117; color: #c9d1d9; padding: 2rem; }}
  h1 {{ color: #58a6ff; }} h2 {{ color: #8b949e; font-size: 0.95rem; }}
  table {{ border-collapse: collapse; width: 100%; margin-top: 1rem; }}
  th {{ background: #161b22; color: #58a6ff; padding: 0.5rem 1rem; text-align: left; }}
  td {{ padding: 0.4rem 1rem; border-bottom: 1px solid #21262d; }}
  tr.warn {{ background: #1a1200; }}
  tr.ok {{ background: #0d1117; }}
  tr.warn-detail td {{ background: #1a1200; color: #d29922; font-size: 0.85rem; padding: 0 1rem 0.5rem 2rem; }}
  .over {{ color: #f85149; font-weight: bold; }}
  ul.warn-list {{ margin: 0; padding-left: 1rem; }}
  .meta {{ color: #8b949e; margin-bottom: 1rem; }}
  .badge {{ display: inline-block; padding: 0.2rem 0.5rem; border-radius: 4px; font-size: 0.8rem; }}
</style>
</head>
<body>
<h1>📊 Sanctifier — Contract Complexity Report</h1>
<div class="meta">
  <b>Contract:</b> {path} &nbsp;|&nbsp;
  <b>Dependencies:</b> {deps} &nbsp;|&nbsp;
  <b>Functions:</b> {fn_count}
</div>
<table>
  <thead>
    <tr>
      <th>Function</th>
      <th>Cyclomatic CC</th>
      <th>Params</th>
      <th>Nesting</th>
      <th>LOC</th>
      <th>Status</th>
    </tr>
  </thead>
  <tbody>
    {rows}
  </tbody>
</table>
<p style="color:#8b949e;margin-top:1rem;font-size:0.8rem;">
  Thresholds: CC &gt; 10 &nbsp;|&nbsp; Params &gt; 5 &nbsp;|&nbsp; Nesting &gt; 4 &nbsp;|&nbsp; LOC &gt; 50
</p>
</body>
</html>"#,
        path = metrics.contract_path,
        deps = metrics.dependency_count,
        fn_count = metrics.functions.len(),
        rows = rows,
    )
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn count_loc(token_str: &str) -> usize {
    token_str.lines().count()
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max { s.to_string() }
    else { format!("{}…", &s[..max - 1]) }
}