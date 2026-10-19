//! S028 — Detect Soroban SDK v22 deprecated usage patterns.
//!
//! Soroban SDK v22 removed or renamed several storage and deployment APIs.
//! This rule flags the three most-common patterns that break on upgrade:
//!
//! | Deprecated (≤ v21)                               | Replacement (v22+)                                      |
//! |--------------------------------------------------|---------------------------------------------------------|
//! | `storage().*.bump(ledgers)`                      | `storage().*.extend_ttl(min_ledgers, max_ledgers)`      |
//! | `RawVal` type                                    | `Val`                                                   |
//! | `env.deployer().deploy(wasm_hash, salt)`         | `env.deployer().with_address(id,salt).deploy_v2(hash,..)` |
//!
//! ## SDK version gate
//!
//! The rule is a no-op when the caller supplies a detected SDK major version
//! that is below 22.  When no version is supplied (the default constructed via
//! [`DeprecatedSdkUsageRule::new`]), the rule always emits — that is the safe,
//! conservative choice for projects that haven't pinned a version.
//!
//! Callers with access to `Cargo.toml` can wire the gate up with:
//! ```no_run
//! use std::path::Path;
//! use sanctifier_core::sdk_version::{detect_sdk_version, parse_major_version};
//! use sanctifier_core::rules::deprecated_sdk_usage::DeprecatedSdkUsageRule;
//!
//! let info = detect_sdk_version(Path::new("Cargo.toml"));
//! let major = info.version.as_deref().and_then(parse_major_version);
//! let rule = DeprecatedSdkUsageRule::with_sdk_major(major);
//! ```

use crate::rules::{Rule, RuleViolation, Severity};
use syn::spanned::Spanned;
use syn::visit::Visit;
use syn::{parse_str, File};

/// Minimum SDK major version at which these patterns are deprecated/removed.
const DEPRECATED_FROM_MAJOR: u32 = 22;

/// Rule that flags Soroban SDK APIs removed or renamed in SDK v22.
pub struct DeprecatedSdkUsageRule {
    /// Detected SDK major version from the project's Cargo.toml.
    /// `None` means unknown — the rule emits conservatively.
    detected_sdk_major: Option<u32>,
}

impl DeprecatedSdkUsageRule {
    /// Create a new instance.  Emits warnings regardless of SDK version (conservative).
    pub fn new() -> Self {
        Self {
            detected_sdk_major: None,
        }
    }

    /// Create an instance scoped to a specific detected SDK major version.
    /// Warnings are suppressed when `sdk_major < 22`.
    pub fn with_sdk_major(sdk_major: Option<u32>) -> Self {
        Self {
            detected_sdk_major: sdk_major,
        }
    }

    /// Returns true when the check should run given the detected SDK version.
    fn should_check(&self) -> bool {
        match self.detected_sdk_major {
            None => true,
            Some(major) => major >= DEPRECATED_FROM_MAJOR,
        }
    }
}

impl Default for DeprecatedSdkUsageRule {
    fn default() -> Self {
        Self::new()
    }
}

impl Rule for DeprecatedSdkUsageRule {
    fn name(&self) -> &str {
        "deprecated_sdk_usage"
    }

    fn description(&self) -> &str {
        "Detects Soroban SDK APIs removed or renamed in SDK v22: \
         storage bump(), RawVal type, and deployer().deploy()"
    }

    fn check(&self, source: &str) -> Vec<RuleViolation> {
        if !self.should_check() {
            return vec![];
        }

        let file = match parse_str::<File>(source) {
            Ok(f) => f,
            Err(_) => return vec![],
        };

        let mut visitor = DeprecatedVisitor {
            violations: Vec::new(),
            current_fn: None,
            test_mod_depth: 0,
        };
        visitor.visit_file(&file);

        visitor
            .violations
            .into_iter()
            .map(|v| {
                RuleViolation::new(self.name(), Severity::Warning, v.message, v.location)
                    .with_suggestion(v.suggestion)
            })
            .collect()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

// ── Internal visitor ──────────────────────────────────────────────────────────

struct PendingViolation {
    message: String,
    location: String,
    suggestion: String,
}

struct DeprecatedVisitor {
    violations: Vec<PendingViolation>,
    current_fn: Option<String>,
    test_mod_depth: u32,
}

impl<'ast> Visit<'ast> for DeprecatedVisitor {
    // ── Skip #[cfg(test)] modules ────────────────────────────────────────────
    fn visit_item_mod(&mut self, node: &'ast syn::ItemMod) {
        if is_cfg_test(&node.attrs) {
            self.test_mod_depth += 1;
            syn::visit::visit_item_mod(self, node);
            self.test_mod_depth -= 1;
        } else {
            syn::visit::visit_item_mod(self, node);
        }
    }

    fn visit_impl_item_fn(&mut self, node: &'ast syn::ImplItemFn) {
        if self.test_mod_depth > 0 || has_test_attr(&node.attrs) {
            return;
        }
        let prev = self.current_fn.take();
        self.current_fn = Some(node.sig.ident.to_string());
        syn::visit::visit_impl_item_fn(self, node);
        self.current_fn = prev;
    }

    fn visit_item_fn(&mut self, node: &'ast syn::ItemFn) {
        if self.test_mod_depth > 0 || has_test_attr(&node.attrs) {
            return;
        }
        let prev = self.current_fn.take();
        self.current_fn = Some(node.sig.ident.to_string());
        syn::visit::visit_item_fn(self, node);
        self.current_fn = prev;
    }

    // ── Pattern 1: storage.*.bump(ledgers) ──────────────────────────────────
    fn visit_expr_method_call(&mut self, node: &'ast syn::ExprMethodCall) {
        let method = node.method.to_string();

        if let Some(fn_name) = self.current_fn.clone() {
            // bump() is the old TTL-extension API removed in SDK v22.
            if method == "bump" {
                let receiver = quote::quote!(#node.receiver).to_string();
                if receiver.contains("storage")
                    || receiver.contains("persistent")
                    || receiver.contains("temporary")
                    || receiver.contains("instance")
                {
                    let line = node.span().start().line;
                    self.violations.push(PendingViolation {
                        message: format!(
                            "Deprecated SDK v22: `storage().*.bump()` was removed — \
                             use `extend_ttl()` instead (in `{fn_name}` at line {line})"
                        ),
                        location: format!("{fn_name}:{line}"),
                        suggestion: "Replace `.bump(ledgers)` with \
                            `.extend_ttl(min_ledgers_to_live, max_ledgers_to_live)`. \
                            Both the minimum and maximum ledger bounds must be provided."
                            .to_string(),
                    });
                }
            }

            // Pattern 3: env.deployer().deploy(wasm_hash, salt) removed in v22.
            if method == "deploy" {
                let receiver_str = quote::quote!(#node.receiver).to_string();
                if receiver_str.contains("deployer") {
                    let line = node.span().start().line;
                    self.violations.push(PendingViolation {
                        message: format!(
                            "Deprecated SDK v22: `deployer().deploy()` was removed — \
                             use `deployer().with_address(id, salt).deploy_v2()` \
                             (in `{fn_name}` at line {line})"
                        ),
                        location: format!("{fn_name}:{line}"),
                        suggestion: "Replace `env.deployer().deploy(wasm_hash, salt)` with \
                            `env.deployer().with_address(contract_id, salt).deploy_v2(wasm_hash, ctor_args)` \
                            or `env.deployer().with_current_contract(salt).deploy_v2(wasm_hash, ctor_args)`."
                            .to_string(),
                    });
                }
            }
        }

        syn::visit::visit_expr_method_call(self, node);
    }

    // ── Pattern 2: RawVal type reference ────────────────────────────────────
    fn visit_type_path(&mut self, node: &'ast syn::TypePath) {
        if self.current_fn.is_some() {
            if let Some(segment) = node.path.segments.last() {
                if segment.ident == "RawVal" {
                    let line = node
                        .path
                        .segments
                        .last()
                        .map(|s| s.ident.span().start().line)
                        .unwrap_or(0);
                    let fn_name = self.current_fn.as_deref().unwrap_or("<unknown>");
                    self.violations.push(PendingViolation {
                        message: format!(
                            "Deprecated SDK v22: `RawVal` was renamed to `Val` \
                             (in `{fn_name}` at line {line})"
                        ),
                        location: format!("{fn_name}:{line}"),
                        suggestion: "Replace `RawVal` with `Val` throughout. \
                            `Val` is the unified host value type in Soroban SDK v22+."
                            .to_string(),
                    });
                }
            }
        }
        syn::visit::visit_type_path(self, node);
    }

    // Also detect RawVal in path expressions (e.g. `RawVal::from(...)`)
    fn visit_path(&mut self, node: &'ast syn::Path) {
        if self.current_fn.is_some() {
            if let Some(segment) = node.segments.first() {
                if segment.ident == "RawVal" && node.segments.len() > 1 {
                    let line = segment.ident.span().start().line;
                    let fn_name = self.current_fn.as_deref().unwrap_or("<unknown>");
                    self.violations.push(PendingViolation {
                        message: format!(
                            "Deprecated SDK v22: `RawVal` was renamed to `Val` \
                             (in `{fn_name}` at line {line})"
                        ),
                        location: format!("{fn_name}:{line}"),
                        suggestion: "Replace `RawVal` with `Val` throughout. \
                            `Val` is the unified host value type in Soroban SDK v22+."
                            .to_string(),
                    });
                }
            }
        }
        syn::visit::visit_path(self, node);
    }
}

fn has_test_attr(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|a| a.path().is_ident("test"))
}

fn is_cfg_test(attrs: &[syn::Attribute]) -> bool {
    attrs
        .iter()
        .any(|a| a.path().is_ident("cfg") && quote::quote!(#a).to_string().contains("test"))
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn rule() -> DeprecatedSdkUsageRule {
        DeprecatedSdkUsageRule::new()
    }

    // ── Pattern 1: bump() ─────────────────────────────────────────────────────

    #[test]
    fn test_detect_storage_bump_persistent() {
        let source = r#"
            fn store(env: Env, key: Symbol, val: u64) {
                env.storage().persistent().set(&key, &val);
                env.storage().persistent().bump(100);
            }
        "#;
        let v = rule().check(source);
        assert_eq!(v.len(), 1);
        assert!(v[0].message.contains("bump"));
        assert!(v[0]
            .suggestion
            .as_deref()
            .unwrap_or("")
            .contains("extend_ttl"));
    }

    #[test]
    fn test_detect_storage_bump_instance() {
        let source = r#"
            fn init(env: Env) {
                env.storage().instance().bump(500);
            }
        "#;
        let v = rule().check(source);
        assert_eq!(v.len(), 1);
        assert!(v[0].message.contains("bump"));
    }

    #[test]
    fn test_detect_storage_bump_temporary() {
        let source = r#"
            fn cache(env: Env) {
                env.storage().temporary().bump(50);
            }
        "#;
        let v = rule().check(source);
        assert_eq!(v.len(), 1);
    }

    #[test]
    fn test_no_false_positive_non_storage_bump() {
        let source = r#"
            fn process(counter: MyCounter) {
                counter.bump();
            }
        "#;
        let v = rule().check(source);
        assert_eq!(v.len(), 0, "bump() on non-storage type must not fire");
    }

    // ── Pattern 2: RawVal ─────────────────────────────────────────────────────

    #[test]
    fn test_detect_rawval_in_type_position() {
        let source = r#"
            fn convert(val: RawVal) -> u64 {
                0
            }
        "#;
        let v = rule().check(source);
        assert!(v.iter().any(|x| x.message.contains("RawVal")));
    }

    #[test]
    fn test_detect_rawval_path_expression() {
        let source = r#"
            fn convert(raw: u64) -> RawVal {
                RawVal::from_u32(raw as u32)
            }
        "#;
        let v = rule().check(source);
        assert!(v.iter().any(|x| x.message.contains("RawVal")));
        let suggestion = v
            .iter()
            .find(|x| x.message.contains("RawVal"))
            .and_then(|x| x.suggestion.as_deref())
            .unwrap_or("");
        assert!(suggestion.contains("Val"));
    }

    #[test]
    fn test_no_false_positive_val_type() {
        let source = r#"
            fn convert(val: Val) -> u64 {
                0
            }
        "#;
        let v = rule().check(source);
        assert_eq!(v.len(), 0, "Val (non-deprecated) must not fire");
    }

    // ── Pattern 3: deployer().deploy() ───────────────────────────────────────

    #[test]
    fn test_detect_deployer_deploy() {
        let source = r#"
            fn deploy_child(env: Env, wasm_hash: BytesN<32>, salt: BytesN<32>) {
                env.deployer().deploy(wasm_hash, salt);
            }
        "#;
        let v = rule().check(source);
        assert_eq!(v.len(), 1);
        assert!(v[0].message.contains("deployer"));
        assert!(v[0]
            .suggestion
            .as_deref()
            .unwrap_or("")
            .contains("deploy_v2"));
    }

    #[test]
    fn test_no_false_positive_deploy_v2() {
        let source = r#"
            fn deploy_child(env: Env, wasm_hash: BytesN<32>, salt: BytesN<32>) {
                env.deployer().with_current_contract(salt).deploy_v2(wasm_hash, ());
            }
        "#;
        let v = rule().check(source);
        assert_eq!(
            v.len(),
            0,
            "deploy_v2 is the correct v22 API and must not fire"
        );
    }

    // ── SDK version gate ─────────────────────────────────────────────────────

    #[test]
    fn test_sdk_v22_emits() {
        let source = r#"
            fn store(env: Env) { env.storage().persistent().bump(100); }
        "#;
        let rule = DeprecatedSdkUsageRule::with_sdk_major(Some(22));
        assert!(!rule.check(source).is_empty(), "SDK v22 should emit");
    }

    #[test]
    fn test_sdk_v21_suppressed() {
        let source = r#"
            fn store(env: Env) { env.storage().persistent().bump(100); }
        "#;
        let rule = DeprecatedSdkUsageRule::with_sdk_major(Some(21));
        assert!(rule.check(source).is_empty(), "SDK v21 must not emit");
    }

    #[test]
    fn test_sdk_unknown_emits_conservatively() {
        let source = r#"
            fn store(env: Env) { env.storage().persistent().bump(100); }
        "#;
        let rule = DeprecatedSdkUsageRule::with_sdk_major(None);
        assert!(
            !rule.check(source).is_empty(),
            "unknown SDK version should emit conservatively"
        );
    }

    #[test]
    fn test_sdk_v23_emits() {
        let source = r#"
            fn store(env: Env) { env.storage().persistent().bump(100); }
        "#;
        let rule = DeprecatedSdkUsageRule::with_sdk_major(Some(23));
        assert!(!rule.check(source).is_empty(), "SDK > 22 should also emit");
    }

    // ── Test / cfg(test) skip ────────────────────────────────────────────────

    #[test]
    fn test_skip_test_functions() {
        let source = r#"
            #[test]
            fn my_test() {
                env.storage().persistent().bump(100);
                let _: RawVal = RawVal::from_u32(0);
            }
        "#;
        assert_eq!(rule().check(source).len(), 0, "#[test] fns must be skipped");
    }

    #[test]
    fn test_skip_cfg_test_module() {
        let source = r#"
            fn real(env: Env) { env.storage().persistent().bump(100); }

            #[cfg(test)]
            mod tests {
                fn helper(env: Env) { env.storage().persistent().bump(200); }
            }
        "#;
        let v = rule().check(source);
        assert_eq!(v.len(), 1, "only real() should fire");
        assert!(v[0].location.contains("real"));
    }
}
