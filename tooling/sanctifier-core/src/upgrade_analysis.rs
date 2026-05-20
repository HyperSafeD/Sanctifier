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
        });
    }

    report
}
