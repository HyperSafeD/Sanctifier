use soroban_sdk::Env;
use syn::{parse_str, File, Item, Type, Fields, Meta};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct SizeWarning {
    pub struct_name: String,
    pub estimated_size: usize,
    pub limit: usize,
}

#[derive(Debug, Serialize, Clone)]
pub struct PanicIssue {
    pub function_name: String,
    pub issue_type: String, // "panic!", "unwrap", "expect"
    pub location: String, // e.g. "struct_name:line" or similar context
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

        gaps
    }

    pub fn scan_panics(&self, source: &str) -> Vec<PanicIssue> {
        let file = match parse_str::<File>(source) {
            Ok(f) => f,
            Err(_) => return vec![],
        };

        let mut issues = Vec::new();

        for item in file.items {
            if let Item::Impl(i) = item {
                for impl_item in &i.items {
                    if let syn::ImplItem::Fn(f) = impl_item {
                        let fn_name = f.sig.ident.to_string();
                        self.check_fn_panics(&f.block, &fn_name, &mut issues);
                    }
                }
            }
        }
        issues
    }

    fn check_fn_panics(&self, block: &syn::Block, fn_name: &str, issues: &mut Vec<PanicIssue>) {
        for stmt in &block.stmts {
            match stmt {
                syn::Stmt::Expr(expr, _) => {
                    self.check_expr_panics(expr, fn_name, issues);
                }
                syn::Stmt::Local(local) => {
                    if let Some(init) = &local.init {
                        self.check_expr_panics(&init.expr, fn_name, issues);
                    }
                }
                _ => {}
            }
        }
    }

    fn check_expr_panics(&self, expr: &syn::Expr, fn_name: &str, issues: &mut Vec<PanicIssue>) {
        match expr {
            syn::Expr::Macro(m) => {
                if m.mac.path.is_ident("panic") {
                    issues.push(PanicIssue {
                        function_name: fn_name.to_string(),
                        issue_type: "panic!".to_string(),
                        location: fn_name.to_string(),
                    });
                }
            }
            syn::Expr::MethodCall(m) => {
                let method_name = m.method.to_string();
                if method_name == "unwrap" || method_name == "expect" {
                    issues.push(PanicIssue {
                        function_name: fn_name.to_string(),
                        issue_type: method_name,
                        location: fn_name.to_string(),
                    });
                }
                self.check_expr_panics(&m.receiver, fn_name, issues);
                for arg in &m.args {
                    self.check_expr_panics(arg, fn_name, issues);
                }
            }
            syn::Expr::Call(c) => {
                for arg in &c.args {
                    self.check_expr_panics(arg, fn_name, issues);
                }
            }
            syn::Expr::Block(b) => {
                self.check_fn_panics(&b.block, fn_name, issues);
            }
            syn::Expr::If(i) => {
                self.check_expr_panics(&i.cond, fn_name, issues);
                self.check_fn_panics(&i.then_branch, fn_name, issues);
                if let Some((_, else_expr)) = &i.else_branch {
                    self.check_expr_panics(else_expr, fn_name, issues);
                }
            }
            syn::Expr::Match(m) => {
                self.check_expr_panics(&m.expr, fn_name, issues);
                for arm in &m.arms {
                    self.check_expr_panics(&arm.body, fn_name, issues);
                }
            }
            _ => {}
        }
    }

    fn check_fn_body(&self, block: &syn::Block, has_mutation: &mut bool, has_auth: &mut bool) {
        for stmt in &block.stmts {
            match stmt {
                syn::Stmt::Expr(expr, _) => {
                    self.check_expr(expr, has_mutation, has_auth);
                }
                syn::Stmt::Local(local) => {
                    if let Some(init) = &local.init {
                        self.check_expr(&init.expr, has_mutation, has_auth);
                    }
                }
                _ => {}
            }
        }
    }

    fn check_expr(&self, expr: &syn::Expr, has_mutation: &mut bool, has_auth: &mut bool) {
        match expr {
            syn::Expr::Call(c) => {
                // Check for require_auth calls
                if let syn::Expr::Path(p) = &*c.func {
                    if let Some(segment) = p.path.segments.last() {
                        let ident = segment.ident.to_string();
                        if ident == "require_auth" || ident == "require_auth_for_args" {
                            *has_auth = true;
                        }
                    }
                }
                for arg in &c.args {
                    self.check_expr(arg, has_mutation, has_auth);
                }
            }
            syn::Expr::MethodCall(m) => {
                let method_name = m.method.to_string();
                if method_name == "set" || method_name == "update" || method_name == "remove" {
                    // Check if it's acting on storage (heuristic: receiver contains "storage")
                    let receiver_str = quote::quote!(#m.receiver).to_string();
                    if receiver_str.contains("storage") {
                        *has_mutation = true;
                    }
                }
                if method_name == "require_auth" || method_name == "require_auth_for_args" {
                    *has_auth = true;
                }
                self.check_expr(&m.receiver, has_mutation, has_auth);
                for arg in &m.args {
                    self.check_expr(arg, has_mutation, has_auth);
                }
            }
            syn::Expr::Block(b) => {
                self.check_fn_body(&b.block, has_mutation, has_auth);
            }
            syn::Expr::If(i) => {
                self.check_expr(&i.cond, has_mutation, has_auth);
                self.check_fn_body(&i.then_branch, has_mutation, has_auth);
                if let Some((_, else_expr)) = &i.else_branch {
                    self.check_expr(else_expr, has_mutation, has_auth);
                }
            }
            syn::Expr::Match(m) => {
                self.check_expr(&m.expr, has_mutation, has_auth);
                for arm in &m.arms {
                    self.check_expr(&arm.body, has_mutation, has_auth);
                }
            }
            // Add more expr types if needed for deep traversal
            _ => {}
        }
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

    #[test]
    fn test_scan_auth_gaps() {
        let analyzer = Analyzer::new(false);
        let source = r#"
            #[contractimpl]
            impl MyContract {
                pub fn set_data(env: Env, val: u32) {
                    env.storage().instance().set(&DataKey::Val, &val);
                }

                pub fn set_data_secure(env: Env, val: u32) {
                    env.require_auth();
                    env.storage().instance().set(&DataKey::Val, &val);
                }

                pub fn get_data(env: Env) -> u32 {
                    env.storage().instance().get(&DataKey::Val).unwrap_or(0)
                }

                pub fn no_storage(env: Env) {
                    let x = 1 + 1;
                }
            }
        "#;
        let gaps = analyzer.scan_auth_gaps(source);
        assert_eq!(gaps.len(), 1);
        assert_eq!(gaps[0], "set_data");
    }

    #[test]
    fn test_scan_panics() {
        let analyzer = Analyzer::new(false);
        let source = r#"
            #[contractimpl]
            impl MyContract {
                pub fn unsafe_fn(env: Env) {
                    panic!("Something went wrong");
                }

                pub fn unsafe_unwrap(env: Env) {
                    let x: Option<u32> = None;
                    let y = x.unwrap();
                }

                pub fn unsafe_expect(env: Env) {
                    let x: Option<u32> = None;
                    let y = x.expect("Failed to get x");
                }

                pub fn safe_fn(env: Env) -> Result<(), u32> {
                    Ok(())
                }
            }
        "#;
        let issues = analyzer.scan_panics(source);
        assert_eq!(issues.len(), 3);
        
        let types: Vec<String> = issues.iter().map(|i| i.issue_type.clone()).collect();
        assert!(types.contains(&"panic!".to_string()));
        assert!(types.contains(&"unwrap".to_string()));
        assert!(types.contains(&"expect".to_string()));
    }
}
