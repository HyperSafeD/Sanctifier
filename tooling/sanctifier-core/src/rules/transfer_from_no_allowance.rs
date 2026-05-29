use crate::rules::{Rule, RuleViolation, Severity};
use syn::{parse_str, File, Item};

/// Rule S023 — detects transfer_from-style functions that move a 'from' balance
/// without checking or decrementing the spender's allowance.
pub struct TransferFromNoAllowanceRule;

impl TransferFromNoAllowanceRule {
    pub fn new() -> Self {
        Self
    }
}

impl Default for TransferFromNoAllowanceRule {
    fn default() -> Self {
        Self::new()
    }
}

impl Rule for TransferFromNoAllowanceRule {
    fn name(&self) -> &str {
        "transfer_from_no_allowance"
    }

    fn description(&self) -> &str {
        "Detects transfer_from-style functions that consume a 'from' balance without checking or decrementing the spender's allowance"
    }

    fn check(&self, source: &str) -> Vec<RuleViolation> {
        let file = match parse_str::<File>(source) {
            Ok(f) => f,
            Err(_) => return vec![],
        };

        let mut violations = Vec::new();

        for item in &file.items {
            if let Item::Impl(impl_block) = item {
                for impl_item in &impl_block.items {
                    if let syn::ImplItem::Fn(method) = impl_item {
                        if !matches!(method.vis, syn::Visibility::Public(_)) {
                            continue;
                        }

                        let fn_name = method.sig.ident.to_string();

                        if !is_transfer_from_candidate(&fn_name, &method.sig) {
                            continue;
                        }

                        let body = quote::quote!(#method.block).to_string();

                        if has_balance_mutation(&body) && !has_allowance_check(&body) {
                            violations.push(
                                RuleViolation::new(
                                    self.name(),
                                    Severity::Error,
                                    format!(
                                        "Function '{}' moves 'from' balance without checking or decrementing allowance — any caller can drain any account",
                                        fn_name
                                    ),
                                    fn_name.clone(),
                                )
                                .with_suggestion(
                                    "Read the spender's allowance for 'from', assert it is >= amount, decrement it, then move the balance. \
                                     Example: let allowance = get_allowance(&e, from.clone(), spender.clone()); \
                                     assert!(allowance >= amount, \"insufficient allowance\"); \
                                     set_allowance(&e, from.clone(), spender.clone(), allowance - amount);"
                                        .to_string(),
                                ),
                            );
                        }
                    }
                }
            }
        }

        violations
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

fn is_transfer_from_candidate(fn_name: &str, sig: &syn::Signature) -> bool {
    // Explicit naming variants
    if fn_name == "transfer_from"
        || fn_name == "transferFrom"
        || fn_name.ends_with("_transfer_from")
        || fn_name.starts_with("transfer_from_")
    {
        return true;
    }

    // Implicit: a transfer/spend function that has a 'spender' or 'operator' param
    // AND a 'from' param, suggesting it acts on behalf of another account.
    if fn_name.contains("transfer") || fn_name.contains("spend") || fn_name.contains("send") {
        let has_spender = sig.inputs.iter().any(|arg| {
            if let syn::FnArg::Typed(pt) = arg {
                if let syn::Pat::Ident(pi) = pt.pat.as_ref() {
                    let n = pi.ident.to_string();
                    return n == "spender" || n == "operator";
                }
            }
            false
        });
        let has_from = sig.inputs.iter().any(|arg| {
            if let syn::FnArg::Typed(pt) = arg {
                if let syn::Pat::Ident(pi) = pt.pat.as_ref() {
                    return pi.ident == "from";
                }
            }
            false
        });
        // 3+ Address params also signals a delegated transfer (spender, from, to)
        let addr_count = count_address_params(sig);
        if has_from && (has_spender || addr_count >= 3) {
            return true;
        }
    }

    false
}

fn count_address_params(sig: &syn::Signature) -> usize {
    sig.inputs
        .iter()
        .filter(|arg| {
            if let syn::FnArg::Typed(pt) = arg {
                if let syn::Type::Path(tp) = pt.ty.as_ref() {
                    return tp
                        .path
                        .segments
                        .last()
                        .map(|s| s.ident == "Address")
                        .unwrap_or(false);
                }
            }
            false
        })
        .count()
}

fn has_balance_mutation(body: &str) -> bool {
    (body.contains(".set(") || body.contains(".update("))
        && (body.to_lowercase().contains("balance") || body.contains("from"))
}

fn has_allowance_check(body: &str) -> bool {
    let lower = body.to_lowercase();
    lower.contains("allowance") || lower.contains("spend_limit") || lower.contains("approved")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flags_transfer_from_without_allowance_check() {
        let rule = TransferFromNoAllowanceRule::new();
        let source = r#"
            impl Token {
                pub fn transfer_from(e: Env, _spender: Address, from: Address, to: Address, amount: i128) {
                    let from_bal: i128 = e.storage().persistent().get(&from).unwrap_or(0);
                    e.storage().persistent().set(&from, &(from_bal - amount));
                    let to_bal: i128 = e.storage().persistent().get(&to).unwrap_or(0);
                    e.storage().persistent().set(&to, &(to_bal + amount));
                }
            }
        "#;
        let v = rule.check(source);
        assert!(!v.is_empty(), "missing allowance check must be flagged");
        assert!(v[0].message.contains("transfer_from"));
        assert!(v[0].suggestion.is_some());
    }

    #[test]
    fn no_violation_when_allowance_is_checked() {
        let rule = TransferFromNoAllowanceRule::new();
        let source = r#"
            impl Token {
                pub fn transfer_from(e: Env, spender: Address, from: Address, to: Address, amount: i128) {
                    spender.require_auth();
                    let allowance = get_allowance(&e, from.clone(), spender.clone());
                    assert!(allowance >= amount, "insufficient allowance");
                    set_allowance(&e, from.clone(), spender.clone(), allowance - amount);
                    let from_bal: i128 = e.storage().persistent().get(&from).unwrap_or(0);
                    e.storage().persistent().set(&from, &(from_bal - amount));
                    let to_bal: i128 = e.storage().persistent().get(&to).unwrap_or(0);
                    e.storage().persistent().set(&to, &(to_bal + amount));
                }
            }
        "#;
        let v = rule.check(source);
        assert!(v.is_empty(), "function with allowance check must not be flagged");
    }

    #[test]
    fn private_transfer_from_not_flagged() {
        let rule = TransferFromNoAllowanceRule::new();
        let source = r#"
            impl Token {
                fn transfer_from(e: Env, _spender: Address, from: Address, to: Address, amount: i128) {
                    let from_bal: i128 = e.storage().persistent().get(&from).unwrap_or(0);
                    e.storage().persistent().set(&from, &(from_bal - amount));
                }
            }
        "#;
        let v = rule.check(source);
        assert!(v.is_empty(), "private functions must not be flagged");
    }

    #[test]
    fn simple_two_address_transfer_not_flagged() {
        let rule = TransferFromNoAllowanceRule::new();
        let source = r#"
            impl Token {
                pub fn transfer(e: Env, from: Address, to: Address, amount: i128) {
                    from.require_auth();
                    let from_bal: i128 = e.storage().persistent().get(&from).unwrap_or(0);
                    e.storage().persistent().set(&from, &(from_bal - amount));
                    let to_bal: i128 = e.storage().persistent().get(&to).unwrap_or(0);
                    e.storage().persistent().set(&to, &(to_bal + amount));
                }
            }
        "#;
        let v = rule.check(source);
        assert!(v.is_empty(), "direct transfer with only from+to must not be flagged");
    }

    #[test]
    fn empty_source_produces_no_findings() {
        let rule = TransferFromNoAllowanceRule::new();
        assert!(rule.check("").is_empty());
    }

    #[test]
    fn invalid_source_produces_no_panic() {
        let rule = TransferFromNoAllowanceRule::new();
        assert!(rule.check("not valid rust {{{").is_empty());
    }
}
