use soroban_sdk::Env;
use syn::{parse_str, File, Item, Type, Fields, Meta};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct SizeWarning {
    pub struct_name: String,
    pub estimated_size: usize,
    pub limit: usize,
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

    pub fn check_storage_collisions(&self, _keys: Vec<String>) -> bool {
        // Placeholder for collision detection
        false
    }

    pub fn analyze_ledger_size(&self, source: &str) -> Vec<SizeWarning> {
        let file = match parse_str::<File>(source) {
            Ok(f) => f,
            Err(_) => return vec![], // Return empty if parsing fails at file level
        };
        
        let mut warnings = Vec::new();

        for item in file.items {
            match item {
                Item::Struct(s) => {
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
                Item::Impl(i) => {
                    // Dive into impl blocks to find nested structs or types if necessary
                    // For now, we just skip without panicking
                    for item in i.items {
                        if let syn::ImplItem::Const(_c) = item {
                            // Example of handling items inside impl
                        }
                    }
                }
                Item::Macro(m) => {
                    // Soroban macros like contractimpl! are common at top-level.
                    // We skip them gracefully as they don't define structs for ledger size in a way we analyze here.
                    if m.mac.path.is_ident("contractimpl") {
                        // Known Soroban macro, skip
                    }
                }
                _ => {} // Skip other items like functions, modules, etc.
            }
        }
        warnings
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
    fn test_analyze_with_macros() {
        let analyzer = Analyzer::new(false);
        let source = r#"
            use soroban_sdk::{contract, contractimpl, Env};

            #[contract]
            pub struct MyContract;

            #[contractimpl]
            impl MyContract {
                pub fn hello(env: Env) {}
            }

            #[contracttype]
            pub struct SmallData {
                pub x: u32,
            }

            #[contracttype]
            pub struct BigData {
                pub buffer: Bytes,
                pub large: u128,
            }
        "#;
        let warnings = analyzer.analyze_ledger_size(source);
        // SmallData: 4 bytes
        // BigData: 64 + 16 = 80 bytes
        // Ledger limit is 64000, so no warnings expected by default
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_analyze_with_limit() {
        let mut analyzer = Analyzer::new(false);
        analyzer.ledger_limit = 50;
        let source = r#"
            #[contracttype]
            pub struct ExceedsLimit {
                pub buffer: Bytes, // 64 bytes
            }
        "#;
        let warnings = analyzer.analyze_ledger_size(source);
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].struct_name, "ExceedsLimit");
        assert_eq!(warnings[0].estimated_size, 64);
    }

    #[test]
    fn test_complex_macro_no_panic() {
        let analyzer = Analyzer::new(false);
        let source = r#"
            // A more complex macro that might confuse a naive parser
            macro_rules! complex {
                ($($t:tt)*) => { $($t)* };
            }

            complex! {
                pub struct MyStruct {
                    pub x: u32,
                }
            }

            #[contractimpl]
            impl Contract {
                pub fn test() {
                    let x = symbol_short!("test");
                }
            }
        "#;
        // Should not panic
        let _ = analyzer.analyze_ledger_size(source);
    }
}
