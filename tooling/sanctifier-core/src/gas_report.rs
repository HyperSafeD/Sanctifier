//! Gas report rendering helpers.

use crate::gas_estimator::GasEstimationReport;
use serde::Serialize;
use syn::spanned::Spanned;
use syn::visit::{self, Visit};

/// Aggregate gas report for a contract or workspace scan.
#[derive(Debug, Clone, Serialize, Default)]
pub struct GasReport {
    /// Per-function gas estimates.
    pub functions: Vec<GasEstimationReport>,
    /// Total estimated instruction count across all functions.
    pub total: usize,
}

impl GasReport {
    /// Build a report from individual function estimates.
    pub fn from_estimates(functions: Vec<GasEstimationReport>) -> Self {
        let total = functions
            .iter()
            .map(|function| function.estimated_instructions)
            .sum();

        Self { functions, total }
    }
}

/// Render the gas report as pretty JSON.
pub fn render_json_report(report: &GasReport) -> String {
    serde_json::to_string_pretty(report).unwrap_or_else(|_| "{}".to_string())
}

/// Render the gas report as a plain-text table.
pub fn render_text_report(report: &GasReport, warnings: &[String]) -> String {
    let mut out = String::new();

    for warning in warnings {
        out.push_str(warning);
        out.push('\n');
    }

    out.push_str("Function                 | Estimated instructions\n");
    out.push_str("-------------------------|-----------------------\n");

    for function in &report.functions {
        out.push_str(&format!(
            "{:<25} | {}\n",
            function.function_name, function.estimated_instructions
        ));
    }

    out.push_str(&format!("Total                    | {}\n", report.total));
    out
}

/// Detect likely unbounded loops in a Rust source file.
pub fn detect_unbounded_loop_warnings(source: &str) -> Vec<String> {
    let file = match syn::parse_file(source) {
        Ok(file) => file,
        Err(_) => return vec![],
    };

    let mut visitor = LoopWarningVisitor { warnings: vec![] };
    visitor.visit_file(&file);
    visitor.warnings
}

struct LoopWarningVisitor {
    warnings: Vec<String>,
}

impl<'ast> Visit<'ast> for LoopWarningVisitor {
    fn visit_expr_while(&mut self, node: &'ast syn::ExprWhile) {
        self.warnings.push(format!(
            "[WARN] line {}: while-loop may be unbounded and inflate gas costs",
            node.span().start().line
        ));
        visit::visit_expr_while(self, node);
    }

    fn visit_expr_loop(&mut self, node: &'ast syn::ExprLoop) {
        self.warnings.push(format!(
            "[WARN] line {}: loop may be unbounded and inflate gas costs",
            node.span().start().line
        ));
        visit::visit_expr_loop(self, node);
    }
}
