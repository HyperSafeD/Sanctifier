use crate::StorageCollisionIssue;
use quote::quote;
use std::collections::HashMap;
use syn::spanned::Spanned;
use syn::{
    visit::{self, Visit},
    Expr, ExprCall, ExprMacro, ExprMethodCall, ItemConst, Lit,
};

const STORAGE_OPS: &[&str] = &["get", "set", "has", "remove", "update", "try_update"];

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum SorobanStorageType {
    Instance,
    Persistent,
    Temporary,
    Unknown,
}

impl SorobanStorageType {
    fn as_str(self) -> &'static str {
        match self {
            Self::Instance => "instance",
            Self::Persistent => "persistent",
            Self::Temporary => "temporary",
            Self::Unknown => "unknown",
        }
    }
}

pub struct StorageVisitor {
    pub collisions: Vec<StorageCollisionIssue>,
    keys: HashMap<(SorobanStorageType, String), Vec<KeyInfo>>,
}

#[derive(Clone)]
struct KeyInfo {
    key_type: String,
    location: String,
    line: usize,
}

impl StorageVisitor {
    pub fn new() -> Self {
        Self {
            collisions: Vec::new(),
            keys: HashMap::new(),
        }
    }

    fn add_key(
        &mut self,
        value: String,
        key_type: String,
        storage_type: SorobanStorageType,
        location: String,
        line: usize,
    ) {
        let info = KeyInfo {
            key_type,
            location,
            line,
        };
        self.keys
            .entry((storage_type, value))
            .or_default()
            .push(info);
    }

    pub fn final_check(&mut self) {
        for ((storage_type, value), infos) in &self.keys {
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
                        key_type: format!("{} ({})", current.key_type, storage_type.as_str()),
                        location: format!("{}:{}", current.location, current.line),
                        message: format!(
                            "Potential {} storage key collision: value '{}' is also used in: {}",
                            storage_type.as_str(),
                            value,
                            others.join(", ")
                        ),
                    });
                }
            }
        }
    }

    fn parse_storage_type_from_expr(expr: &Expr) -> SorobanStorageType {
        match expr {
            Expr::MethodCall(method_call) => {
                let method_name = method_call.method.to_string();
                if method_name == "instance" {
                    SorobanStorageType::Instance
                } else if method_name == "persistent" {
                    SorobanStorageType::Persistent
                } else if method_name == "temporary" {
                    SorobanStorageType::Temporary
                } else {
                    Self::parse_storage_type_from_expr(&method_call.receiver)
                }
            }
            Expr::Reference(reference) => Self::parse_storage_type_from_expr(&reference.expr),
            Expr::Paren(paren) => Self::parse_storage_type_from_expr(&paren.expr),
            _ => SorobanStorageType::Unknown,
        }
    }

    fn extract_key_value_expr(expr: &Expr) -> Option<String> {
        match expr {
            Expr::Lit(expr_lit) => match &expr_lit.lit {
                Lit::Str(lit_str) => Some(lit_str.value()),
                Lit::Int(lit_int) => Some(lit_int.base10_digits().to_string()),
                Lit::Bool(lit_bool) => Some(lit_bool.value.to_string()),
                _ => None,
            },
            Expr::Path(expr_path) => Some(quote!(#expr_path).to_string()),
            Expr::Reference(reference) => Self::extract_key_value_expr(&reference.expr),
            Expr::Paren(paren) => Self::extract_key_value_expr(&paren.expr),
            Expr::Call(call) => Some(quote!(#call).to_string()),
            Expr::MethodCall(method_call) => Some(quote!(#method_call).to_string()),
            Expr::Macro(expr_macro) => Some(quote!(#expr_macro).to_string()),
            _ => None,
        }
    }
}

impl<'ast> Visit<'ast> for StorageVisitor {
    fn visit_item_const(&mut self, i: &'ast ItemConst) {
        let key_name = i.ident.to_string();
        if let Expr::Lit(expr_lit) = &*i.expr {
            if let Lit::Str(lit_str) = &expr_lit.lit {
                let val = lit_str.value();
                self.add_key(
                    val,
                    "const".to_string(),
                    SorobanStorageType::Unknown,
                    key_name,
                    i.span().start().line,
                );
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
                                SorobanStorageType::Unknown,
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
                SorobanStorageType::Unknown,
                "inline".to_string(),
                i.span().start().line,
            );
        }
        visit::visit_expr_macro(self, i);
    }

    fn visit_expr_method_call(&mut self, i: &'ast ExprMethodCall) {
        let method_name = i.method.to_string();
        if STORAGE_OPS.contains(&method_name.as_str()) {
            let storage_type = Self::parse_storage_type_from_expr(&i.receiver);
            if let Some(first_arg) = i.args.first() {
                if let Some(key_value) = Self::extract_key_value_expr(first_arg) {
                    self.add_key(
                        key_value,
                        format!("storage::{}", method_name),
                        storage_type,
                        "storage-op".to_string(),
                        i.span().start().line,
                    );
                }
            }
        }
        visit::visit_expr_method_call(self, i);
    }
}
