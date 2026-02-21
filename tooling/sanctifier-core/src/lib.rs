use soroban_sdk::Env;
use syn::{parse_str, File, Item, Type, Fields, Meta, ExprMacro, ExprMethodCall, Macro};
use syn::visit::{self, Visit};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct SizeWarning {
    pub struct_name: String,
    pub estimated_size: usize,
    pub limit: usize,
}

#[derive(Debug, Serialize, Clone, Copy)]
pub enum PatternType {
    Panic,
    Unwrap,
    Expect,
}

#[derive(Debug, Serialize)]
pub struct UnsafePattern {
    pub pattern_type: PatternType,
    pub line: usize,
    pub snippet: String,
}

struct UnsafeVisitor {
    patterns: Vec<UnsafePattern>,
}

impl<'ast> Visit<'ast> for UnsafeVisitor {
    fn visit_macro(&mut self, i: &'ast Macro) {
        if i.path.is_ident("panic") {
            self.patterns.push(UnsafePattern {
                pattern_type: PatternType::Panic,
                line: i.path.segments[0].ident.span().start().line,
                snippet: "panic!".to_string(),
            });
        }
        visit::visit_macro(self, i);
    }

    fn visit_expr_method_call(&mut self, i: &'ast ExprMethodCall) {
        let method_name = i.method.to_string();
        if method_name == "unwrap" || method_name == "expect" {
            self.patterns.push(UnsafePattern {
                pattern_type: if method_name == "unwrap" { PatternType::Unwrap } else { PatternType::Expect },
                line: i.method.span().start().line,
                snippet: method_name,
            });
        }
        visit::visit_expr_method_call(self, i);
    }
}

pub struct Analyzer {
    pub strict_mode: bool,
    pub ledger_limit: usize,
}

impl Analyzer {
    pub fn new(strict_mode: bool) -> Self {
        Self { 
            strict_mode,
            ledger_limit: 64000, // Default 64KB warning threshold
        }
    }

    pub fn scan_auth_gaps(&self, _code: &str) -> Vec<String> {
        // Placeholder for AST analysis logic
        vec![]
    }

    pub fn check_storage_collisions(&self, keys: Vec<String>) -> bool {
        // Placeholder for collision detection
        false
    }

    pub fn analyze_ledger_size(&self, source: &str) -> Vec<SizeWarning> {
        let file = match parse_str::<File>(source) {
            Ok(f) => f,
            Err(_) => return vec![],
        };
        
        let mut warnings = Vec::new();

        for item in file.items {
            if let Item::Struct(s) = item {
                let has_contracttype = s.attrs.iter().any(|attr| {
                    match &attr.meta {
                        Meta::Path(path) => path.is_ident("contracttype"),
                        _ => false,
                    }
                });

                if has_contracttype {
                    let size = self.estimate_struct_size(&s);
                    if size > self.ledger_limit || (self.strict_mode && size > self.ledger_limit / 2) {
                        warnings.push(SizeWarning {
                            struct_name: s.ident.to_string(),
                            estimated_size: size,
                            limit: self.ledger_limit,
                        });
                    }
                }
            }
        }
        warnings
    }

    pub fn analyze_unsafe_patterns(&self, source: &str) -> Vec<UnsafePattern> {
        let file = match parse_str::<File>(source) {
            Ok(f) => f,
            Err(_) => return vec![],
        };
        
        let mut visitor = UnsafeVisitor { patterns: Vec::new() };
        visitor.visit_file(&file);
        visitor.patterns
    }

    fn estimate_struct_size(&self, s: &syn::ItemStruct) -> usize {
        let mut total_size = 0;
        match &s.fields {
            Fields::Named(fields) => {
                for field in &fields.named {
                    total_size += self.estimate_type_size(&field.ty);
                }
            }
            Fields::Unnamed(fields) => {
                for field in &fields.unnamed {
                    total_size += self.estimate_type_size(&field.ty);
                }
            }
            Fields::Unit => {}
        }
        total_size
    }

    fn estimate_type_size(&self, ty: &Type) -> usize {
        match ty {
            Type::Path(tp) => {
                if let Some(segment) = tp.path.segments.last() {
                    let ident = segment.ident.to_string();
                    match ident.as_str() {
                        "u32" | "i32" | "bool" => 4,
                        "u64" | "i64" => 8,
                        "u128" | "i128" | "I128" | "U128" => 16,
                        "Address" => 32,
                        "Bytes" | "BytesN" | "String" | "Symbol" => 64,
                        "Vec" | "Map" => 128,
                        _ => 32,
                    }
                } else {
                    8
                }
            }
            _ => 8,
        }
    }
}

pub trait SanctifiedGuard {
    fn check_invariant(&self, env: &Env) -> Result<(), String>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_panic() {
        let source = r#"
            pub fn test() {
                panic!("error");
            }
        "#;
        let analyzer = Analyzer::new(false);
        let patterns = analyzer.analyze_unsafe_patterns(source);
        assert_eq!(patterns.len(), 1);
        assert_eq!(patterns[0].snippet, "panic!");
    }

    #[test]
    fn test_find_unwrap_expect() {
        let source = r#"
            pub fn test() {
                let x: Option<i32> = None;
                x.unwrap();
                x.expect("msg");
            }
        "#;
        let analyzer = Analyzer::new(false);
        let patterns = analyzer.analyze_unsafe_patterns(source);
        assert_eq!(patterns.len(), 2);
    }
}
