use crate::StorageCollisionIssue;
use quote::quote;
use std::collections::HashMap;
use syn::spanned::Spanned;
use syn::{
    visit::{self, Visit},
    Expr, ExprCall, ExprMacro, ItemConst, Lit,
};

pub struct StorageVisitor {
    pub collisions: Vec<StorageCollisionIssue>,
    pub keys: HashMap<String, Vec<KeyInfo>>,
}

#[derive(Clone)]
pub struct KeyInfo {
    pub _value: String,
    pub key_type: String,
    pub location: String,
    pub line: usize,
}

impl StorageVisitor {
    pub fn new() -> Self {
        Self {
            collisions: Vec::new(),
            keys: HashMap::new(),
        }
    }

    fn add_key(&mut self, value: String, key_type: String, location: String, line: usize) {
        let info = KeyInfo {
            _value: value.clone(),
            key_type,
            location,
            line,
        };
        self.keys.entry(value).or_default().push(info);
    }

    pub fn final_check(&mut self) {
        for (value, infos) in &self.keys {
            if infos.len() > 1 {
                for i in 0..infos.len() {
                    let current = &infos[i];
                    let others: Vec<String> = infos
                        .iter()
                        .enumerate()
                        .filter(|(idx, _)| *idx != i)
                        .map(|(_, info)| format!("{} (line {})", info.location, info.line))
                        .collect();

                    self.collisions.push(StorageCollisionIssue {
                        key_value: value.clone(),
                        key_type: current.key_type.clone(),
                        location: format!("{}:{}", current.location, current.line),
                        message: format!(
                            "Potential storage key collision: value '{}' is also used in: {}",
                            value,
                            others.join(", ")
                        ),
                    });
                }
            }
        }
    }
}

impl<'ast> Visit<'ast> for StorageVisitor {
    fn visit_item_const(&mut self, i: &'ast ItemConst) {
        let key_name = i.ident.to_string();
        if let Expr::Lit(expr_lit) = &*i.expr {
            if let Lit::Str(lit_str) = &expr_lit.lit {
                let val = lit_str.value();
                self.add_key(val, "const".to_string(), key_name, i.span().start().line);
            }
        }
        visit::visit_item_const(self, i);
    }

    fn visit_expr_call(&mut self, i: &'ast ExprCall) {
        // Look for Symbol::new(&env, "...")
        if let Expr::Path(expr_path) = &*i.func {
            let path = &expr_path.path;
            if path.segments.len() >= 2 {
                let seg1 = &path.segments[0].ident;
                let seg2 = &path.segments[1].ident;
                if seg1 == "Symbol" && seg2 == "new" && i.args.len() >= 2 {
                    if let Expr::Lit(expr_lit) = &i.args[1] {
                        if let Lit::Str(lit_str) = &expr_lit.lit {
                            let val = lit_str.value();
                            self.add_key(
                                val,
                                "Symbol::new".to_string(),
                                "inline".to_string(),
                                i.span().start().line,
                            );
                        }
                    }
                }
            }
        }
        visit::visit_expr_call(self, i);
    }

    fn visit_expr_macro(&mut self, i: &'ast ExprMacro) {
        let macro_name = i
            .mac
            .path
            .segments
            .last()
            .map(|s| s.ident.to_string())
            .unwrap_or_default();
        if macro_name == "symbol_short" {
            let tokens = &i.mac.tokens;
            let token_str = quote!(#tokens).to_string();
            // symbol_short!("...") -> token_str might be "\" ... \""
            let val = token_str.trim_matches('"').to_string();
            self.add_key(
                val,
                "symbol_short!".to_string(),
                "inline".to_string(),
                i.span().start().line,
            );
        }
        visit::visit_expr_macro(self, i);
    }
}
