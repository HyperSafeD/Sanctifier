use anyhow::Context;
use clap::Args;
use std::fmt::Write as FmtWrite;
use std::fs;
use std::path::PathBuf;
use syn::{parse_str, visit::Visit, Expr, File};

#[derive(Args, Debug)]
pub struct CallgraphArgs {
    /// Path to the Rust source file to analyze
    pub path: PathBuf,

    /// Output path for the generated DOT file
    #[arg(short, long, default_value = "callgraph.dot")]
    pub output: PathBuf,
}

struct CallgraphEdge {
    contract: String,
    caller_fn: String,
    target: String,
    fn_symbol: Option<String>,
}

struct CallgraphVisitor {
    current_contract: String,
    current_fn: String,
    edges: Vec<CallgraphEdge>,
}

impl CallgraphVisitor {
    fn new() -> Self {
        Self {
            current_contract: String::new(),
            current_fn: String::new(),
            edges: Vec::new(),
        }
    }
}

impl<'ast> Visit<'ast> for CallgraphVisitor {
    fn visit_item_impl(&mut self, i: &'ast syn::ItemImpl) {
        // Extract the contract name from `impl ContractName { ... }`
        if let syn::Type::Path(type_path) = &*i.self_ty {
            if let Some(seg) = type_path.path.segments.last() {
                self.current_contract = seg.ident.to_string();
            }
        }
        syn::visit::visit_item_impl(self, i);
    }

    fn visit_impl_item_fn(&mut self, i: &'ast syn::ImplItemFn) {
        self.current_fn = i.sig.ident.to_string();
        syn::visit::visit_impl_item_fn(self, i);
    }

    fn visit_expr_method_call(&mut self, i: &'ast syn::ExprMethodCall) {
        let method = i.method.to_string();
        if method == "invoke_contract" || method == "invoke_contract_check" {
            let target = if !i.args.is_empty() {
                expr_to_str(&i.args[0])
            } else {
                "<unknown>".to_string()
            };
            let fn_symbol = if i.args.len() >= 2 {
                Some(expr_to_str(&i.args[1]))
            } else {
                None
            };
            self.edges.push(CallgraphEdge {
                contract: self.current_contract.clone(),
                caller_fn: self.current_fn.clone(),
                target,
                fn_symbol,
            });
        }
        syn::visit::visit_expr_method_call(self, i);
    }
}

fn expr_to_str(expr: &Expr) -> String {
    match expr {
        Expr::Path(p) => p
            .path
            .segments
            .last()
            .map(|s| s.ident.to_string())
            .unwrap_or_else(|| "<unknown>".to_string()),
        Expr::Reference(r) => expr_to_str(&r.expr),
        Expr::Paren(p) => expr_to_str(&p.expr),
        _ => quote::quote!(#expr).to_string(),
    }
}

pub fn exec(args: CallgraphArgs) -> anyhow::Result<()> {
    let source = fs::read_to_string(&args.path)
        .with_context(|| format!("failed to read {}", args.path.display()))?;

    let file: File =
        parse_str(&source).with_context(|| format!("failed to parse {}", args.path.display()))?;

    let mut visitor = CallgraphVisitor::new();
    syn::visit::visit_file(&mut visitor, &file);

    let mut dot = String::from("digraph ContractCallGraph {\n");
    for edge in &visitor.edges {
        let label = match &edge.fn_symbol {
            Some(sym) => format!(" [label=\"{}: {}\"]", edge.caller_fn, sym),
            None => format!(" [label=\"{}\"]", edge.caller_fn),
        };
        writeln!(
            dot,
            "    \"{}\" -> \"{}\"{}",
            edge.contract, edge.target, label
        )?;
    }
    dot.push_str("}\n");

    fs::write(&args.output, &dot)
        .with_context(|| format!("failed to write {}", args.output.display()))?;

    println!(
        "Call graph written to {} ({} edge(s))",
        args.output.display(),
        visitor.edges.len()
    );
    Ok(())
}
