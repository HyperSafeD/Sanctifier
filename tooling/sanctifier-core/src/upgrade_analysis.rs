//! Upgrade and admin-pattern analysis.

use crate::{UpgradeCategory, UpgradeFinding, UpgradeReport};
use syn::parse_str;

fn has_contracttype(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|attr| {
        matches!(&attr.meta, syn::Meta::Path(path) if path.is_ident("contracttype") || path.segments.iter().any(|s| s.ident == "contracttype"))
    })
}

/// Check if a function name indicates an upgrade or admin operation.
pub fn is_upgrade_or_admin_fn(name: &str) -> bool {
    let lower = name.to_lowercase();
    matches!(
        lower.as_str(),
        "set_admin"
            | "upgrade"
            | "set_authorized"
            | "deploy"
            | "update_admin"
            | "transfer_admin"
            | "change_admin"
    ) || (lower.contains("upgrade") && (lower.contains("contract") || lower.contains("wasm")))
}

/// Check if a function name indicates an initialization operation.
pub fn is_init_fn(name: &str) -> bool {
    let lower = name.to_lowercase();
    lower == "initialize" || lower == "init" || lower == "initialise"
}

/// Check if a function block contains an early-return guard against re-initialization.
fn fn_has_reinit_guard(block: &syn::Block) -> bool {
    for stmt in &block.stmts {
        match stmt {
            syn::Stmt::Expr(expr, _) => {
                if expr_has_storage_guard(expr) {
                    return true;
                }
            }
            syn::Stmt::Local(local) => {
                if let Some(init) = &local.init {
                    if expr_has_storage_guard(&init.expr) {
                        return true;
                    }
                }
            }
            _ => {}
        }
    }
    false
}

fn expr_has_storage_guard(expr: &syn::Expr) -> bool {
    match expr {
        syn::Expr::If(i) => {
            let cond_str = quote::quote!(#i.cond).to_string();
            let has_storage_check = cond_str.contains(".has(")
                || cond_str.contains(".get(")
                || cond_str.contains(".try_get(");
            if !has_storage_check {
                return expr_has_storage_guard(&i.cond)
                    || block_has_early_exit(&i.then_branch);
            }
            if block_has_early_exit(&i.then_branch) {
                return true;
            }
            if let Some((_, else_expr)) = &i.else_branch {
                if expr_has_early_exit(else_expr) {
                    return true;
                }
            }
            true
        }
        syn::Expr::Block(b) => fn_has_reinit_guard(&b.block),
        syn::Expr::Unary(u) => expr_has_storage_guard(&u.expr),
        syn::Expr::Paren(p) => expr_has_storage_guard(&p.expr),
        syn::Expr::Binary(b) => expr_has_storage_guard(&b.left) || expr_has_storage_guard(&b.right),
        _ => false,
    }
}

fn block_has_early_exit(block: &syn::Block) -> bool {
    for stmt in &block.stmts {
        match stmt {
            syn::Stmt::Expr(expr, _) => {
                if expr_has_early_exit(expr) {
                    return true;
                }
            }
            syn::Stmt::Macro(m) => {
                if m.mac.path.is_ident("panic") {
                    return true;
                }
            }
            _ => {}
        }
    }
    false
}

fn expr_has_early_exit(expr: &syn::Expr) -> bool {
    match expr {
        syn::Expr::Return(_) => true,
        syn::Expr::Macro(m) => m.mac.path.is_ident("panic"),
        syn::Expr::Block(b) => block_has_early_exit(&b.block),
        syn::Expr::If(i) => {
            block_has_early_exit(&i.then_branch)
                || i.else_branch
                    .as_ref()
                    .map(|(_, e)| expr_has_early_exit(e))
                    .unwrap_or(false)
        }
        _ => false,
    }
}

/// Analyze upgrade/admin patterns and return an [`UpgradeReport`].
pub fn analyze_upgrade_patterns(source: &str) -> UpgradeReport {
    let file = match parse_str::<syn::File>(source) {
        Ok(file) => file,
        Err(_) => return UpgradeReport::empty(),
    };

    let mut report = UpgradeReport::empty();

    for item in &file.items {
        match item {
            syn::Item::Struct(s) if has_contracttype(&s.attrs) => {
                report.storage_types.push(s.ident.to_string());
            }
            syn::Item::Enum(e) if has_contracttype(&e.attrs) => {
                report.storage_types.push(e.ident.to_string());
            }
            syn::Item::Impl(i) => {
                for impl_item in &i.items {
                    if let syn::ImplItem::Fn(f) = impl_item {
                        if let syn::Visibility::Public(_) = f.vis {
                            let fn_name = f.sig.ident.to_string();
                            if is_init_fn(&fn_name) {
                                report.init_functions.push(fn_name.clone());
                                let has_guard = fn_has_reinit_guard(&f.block);
                                let severity = if has_guard { "medium" } else { "critical" };
                                let message = if has_guard {
                                    format!("Initialization function '{}' detected (with re-init guard)", fn_name)
                                } else {
                                    format!("Initialization function '{}' is callable more than once — add re-init guard", fn_name)
                                };
                                report.findings.push(UpgradeFinding {
                                    category: UpgradeCategory::InitPattern,
                                    function_name: Some(fn_name.clone()),
                                    location: fn_name.clone(),
                                    message,
                                    suggestion: "Guard init with an early return when storage already has an initialization flag: if env.storage().instance().has(&DataKey::IsInit) { return; }".to_string(),
                                    severity,
                                });
                            }
                            if is_upgrade_or_admin_fn(&fn_name) {
                                report.upgrade_mechanisms.push(fn_name.clone());
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    if !report.upgrade_mechanisms.is_empty() {
        report.findings.push(UpgradeFinding {
            category: UpgradeCategory::Governance,
            function_name: report.upgrade_mechanisms.first().cloned(),
            location: report
                .upgrade_mechanisms
                .first()
                .cloned()
                .unwrap_or_else(|| "<unknown>".to_string()),
            message: "Upgrade/admin mechanism detected".to_string(),
            suggestion: "Ensure upgrade/admin functions are properly access-controlled (e.g. require_auth) and consider timelocks/governance.".to_string(),
            severity: "high".to_string(),
        });
    }

    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::UpgradeCategory;

    #[test]
    fn test_is_init_fn_variants() {
        assert!(is_init_fn("initialize"));
        assert!(is_init_fn("init"));
        assert!(is_init_fn("initialise"));
        assert!(is_init_fn("Initialize"));
        assert!(is_init_fn("INIT"));
        assert!(!is_init_fn("set_admin"));
        assert!(!is_init_fn("balance"));
    }

    #[test]
    fn test_is_upgrade_or_admin_fn() {
        assert!(is_upgrade_or_admin_fn("set_admin"));
        assert!(is_upgrade_or_admin_fn("upgrade"));
        assert!(is_upgrade_or_admin_fn("deploy"));
        assert!(is_upgrade_or_admin_fn("upgrade_contract"));
        assert!(!is_upgrade_or_admin_fn("transfer"));
    }

    #[test]
    fn test_analyze_detects_init_without_guard() {
        let source = r#"
            #[contractimpl]
            impl Token {
                pub fn initialize(env: Env, admin: Address) {
                    env.storage().instance().set(&DataKey::Admin, &admin);
                }
            }
        "#;
        let report = analyze_upgrade_patterns(source);
        assert_eq!(report.init_functions, vec!["initialize"]);
        assert!(!report.findings.is_empty());
        let init_finding = report.findings.iter().find(|f| f.category == UpgradeCategory::InitPattern).unwrap();
        assert_eq!(init_finding.severity, "critical");
        assert!(init_finding.message.contains("callable more than once"));
    }

    #[test]
    fn test_analyze_detects_init_with_has_guard() {
        let source = r#"
            use soroban_sdk::{contractimpl, Env, Address, Symbol};

            #[contracttype]
            pub enum DataKey { IsInit, Admin }

            #[contractimpl]
            impl Token {
                pub fn initialize(env: Env, admin: Address) {
                    if env.storage().instance().has(&DataKey::IsInit) {
                        return;
                    }
                    env.storage().instance().set(&DataKey::Admin, &admin);
                    env.storage().instance().set(&DataKey::IsInit, &true);
                }
            }
        "#;
        let report = analyze_upgrade_patterns(source);
        let init_finding = report.findings.iter().find(|f| f.category == UpgradeCategory::InitPattern).unwrap();
        assert_eq!(init_finding.severity, "medium");
        assert!(init_finding.message.contains("with re-init guard"));
    }

    #[test]
    fn test_analyze_detects_init_with_get_guard() {
        let source = r#"
            use soroban_sdk::{contractimpl, Env, Address, Symbol};

            #[contracttype]
            pub enum DataKey { IsInit, Admin }

            #[contractimpl]
            impl Token {
                pub fn initialize(env: Env, admin: Address) {
                    if env.storage().instance().get(&DataKey::IsInit).is_some() {
                        return;
                    }
                    env.storage().instance().set(&DataKey::Admin, &admin);
                    env.storage().instance().set(&DataKey::IsInit, &true);
                }
            }
        "#;
        let report = analyze_upgrade_patterns(source);
        let init_finding = report.findings.iter().find(|f| f.category == UpgradeCategory::InitPattern).unwrap();
        assert_eq!(init_finding.severity, "medium");
    }

    #[test]
    fn test_analyze_detects_init_with_negated_has_guard() {
        let source = r#"
            use soroban_sdk::{contractimpl, Env, Address, Symbol};

            #[contracttype]
            pub enum DataKey { IsInit, Admin }

            #[contractimpl]
            impl Token {
                pub fn initialize(env: Env, admin: Address) {
                    if !env.storage().instance().has(&DataKey::IsInit) {
                        env.storage().instance().set(&DataKey::Admin, &admin);
                        env.storage().instance().set(&DataKey::IsInit, &true);
                    }
                }
            }
        "#;
        let report = analyze_upgrade_patterns(source);
        let init_finding = report.findings.iter().find(|f| f.category == UpgradeCategory::InitPattern).unwrap();
        assert_eq!(init_finding.severity, "medium");
    }

    #[test]
    fn test_analyze_init_no_false_positive_on_auth() {
        // require_auth alone is NOT a re-init guard
        let source = r#"
            #[contractimpl]
            impl Token {
                pub fn initialize(env: Env, admin: Address) {
                    admin.require_auth();
                    env.storage().instance().set(&DataKey::Admin, &admin);
                }
            }
        "#;
        let report = analyze_upgrade_patterns(source);
        let init_finding = report.findings.iter().find(|f| f.category == UpgradeCategory::InitPattern).unwrap();
        assert_eq!(init_finding.severity, "critical");
    }

    #[test]
    fn test_analyze_multiple_init_functions() {
        let source = r#"
            #[contractimpl]
            impl Token {
                pub fn initialize(env: Env) {
                    // No guard — critical
                }
                pub fn reinit(env: Env) {
                    if env.storage().instance().has(&Symbol::new(&env, "init")) {
                        return;
                    }
                }
            }
        "#;
        let report = analyze_upgrade_patterns(source);
        assert_eq!(report.init_functions.len(), 2);
        let critical_findings: Vec<_> = report.findings.iter().filter(|f| f.severity == "critical").collect();
        assert_eq!(critical_findings.len(), 1);
        assert!(critical_findings[0].function_name.as_deref() == Some("initialize"));
    }

    #[test]
    fn test_analyze_upgrade_mechanism_sets_severity() {
        let source = r#"
            #[contractimpl]
            impl Token {
                pub fn set_admin(env: Env, new_admin: Address) {
                    env.storage().instance().set(&DataKey::Admin, &new_admin);
                }
            }
        "#;
        let report = analyze_upgrade_patterns(source);
        let gov_finding = report.findings.iter().find(|f| f.category == UpgradeCategory::Governance).unwrap();
        assert_eq!(gov_finding.severity, "high");
    }
}
