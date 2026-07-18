use crate::input_validation::{validate_no_null_bytes, validate_source_size};
use crate::rules::{Patch, Rule, RuleViolation, Severity};
use syn::spanned::Spanned;
use syn::{parse_str, File, Item};

/// Rule that flags public functions modifying state without auth.
pub struct AuthGapRule;

#[derive(Default)]
struct FunctionSecuritySummary {
    has_mutation: bool,
    has_auth: bool,
    has_external_call: bool,
}

impl FunctionSecuritySummary {
    fn has_sensitive_action(&self) -> bool {
        self.has_mutation || self.has_external_call
    }
}

fn is_reserved_soroban_entrypoint(fn_name: &str) -> bool {
    matches!(fn_name, "__constructor" | "__check_auth")
}

impl AuthGapRule {
    /// Create a new instance.
    pub fn new() -> Self {
        Self
    }
}

impl Default for AuthGapRule {
    fn default() -> Self {
        Self::new()
    }
}

impl Rule for AuthGapRule {
    fn name(&self) -> &str {
        "auth_gap"
    }

    fn description(&self) -> &str {
        "Detects public functions that perform privileged storage changes or external contract calls without authentication checks"
    }

    fn check(&self, source: &str) -> Vec<RuleViolation> {
        // Guard: empty source has no findings (fast-path, no parse needed).
        if let Err(e) = validate_source_size(source) {
            if e.code == "EMPTY_SOURCE" {
                return vec![];
            }
            // Source too large — emit a structured diagnostic so CI can surface it.
            return vec![RuleViolation::new(
                self.name(),
                Severity::Error,
                format!("Input rejected by auth_gap rule: {}", e.message),
                "<source>".to_string(),
            )
            .with_suggestion(
                "Split the contract into smaller files and analyse each separately.".to_string(),
            )];
        }

        // Guard: null bytes are never valid in Rust source and indicate binary
        // data or a potential injection attempt.
        if let Err(e) = validate_no_null_bytes(source) {
            return vec![RuleViolation::new(
                self.name(),
                Severity::Error,
                format!("Input rejected by auth_gap rule: {}", e.message),
                "<source>".to_string(),
            )
            .with_suggestion(
                "Ensure the file is saved as UTF-8 text and contains no binary data.".to_string(),
            )];
        }

        let file = match parse_str::<File>(source) {
            Ok(f) => f,
            Err(_) => return vec![],
        };

        let mut gaps = Vec::new();
        for item in &file.items {
            if let Item::Impl(i) = item {
                for impl_item in &i.items {
                    if let syn::ImplItem::Fn(f) = impl_item {
                        if let syn::Visibility::Public(_) = f.vis {
                            let fn_name = f.sig.ident.to_string();
                            if is_reserved_soroban_entrypoint(&fn_name) {
                                continue;
                            }
                            let fn_line = f.sig.ident.span().start().line;
                            let mut summary = FunctionSecuritySummary::default();
                            check_fn_body(&f.block, &mut summary);
                            if summary.has_sensitive_action() && !summary.has_auth {
                                gaps.push(RuleViolation::new(
                                    self.name(),
                                    Severity::Warning,
                                    format!("Function '{}' performs a privileged operation without authentication", fn_name),
                                    format!("{}:{}", fn_name, fn_line),
                                ).with_suggestion("Add require_auth() or require_auth_for_args() before storage operations or external contract calls".to_string()));
                            }
                        }
                    }
                }
            }
        }
        gaps
    }

    fn fix(&self, source: &str) -> Vec<Patch> {
        let file = match parse_str::<File>(source) {
            Ok(f) => f,
            Err(_) => return vec![],
        };

        let mut patches = Vec::new();
        for item in &file.items {
            if let Item::Impl(i) = item {
                for impl_item in &i.items {
                    if let syn::ImplItem::Fn(f) = impl_item {
                        if let syn::Visibility::Public(_) = f.vis {
                            if is_reserved_soroban_entrypoint(&f.sig.ident.to_string()) {
                                continue;
                            }
                            let mut summary = FunctionSecuritySummary::default();
                            check_fn_body(&f.block, &mut summary);
                            if summary.has_sensitive_action() && !summary.has_auth {
                                // Add require_auth() as the first statement in the function
                                if let Some(first_stmt) = f.block.stmts.first() {
                                    let span = first_stmt.span();
                                    patches.push(Patch {
                                        start_line: span.start().line,
                                        start_column: span.start().column,
                                        end_line: span.start().line,
                                        end_column: span.start().column,
                                        replacement: "env.require_auth();\n    ".to_string(),
                                        description: format!(
                                            "Add require_auth() to function '{}'",
                                            f.sig.ident
                                        ),
                                    });
                                } else {
                                    // Empty body, just insert at the start of block
                                    let span = f.block.span();
                                    patches.push(Patch {
                                        start_line: span.start().line,
                                        start_column: span.start().column + 1,
                                        end_line: span.start().line,
                                        end_column: span.start().column + 1,
                                        replacement: "\n        env.require_auth();".to_string(),
                                        description: format!(
                                            "Add require_auth() to function '{}'",
                                            f.sig.ident
                                        ),
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
        patches
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

fn check_fn_body(block: &syn::Block, summary: &mut FunctionSecuritySummary) {
    for stmt in &block.stmts {
        match stmt {
            syn::Stmt::Expr(expr, _) => check_expr(expr, summary),
            syn::Stmt::Local(local) => {
                if let Some(init) = &local.init {
                    check_expr(&init.expr, summary);
                }
            }
            syn::Stmt::Macro(m)
                if m.mac.path.is_ident("require_auth")
                    || m.mac.path.is_ident("require_auth_for_args") =>
            {
                summary.has_auth = true;
            }
            _ => {}
        }
    }
}

fn check_expr(expr: &syn::Expr, summary: &mut FunctionSecuritySummary) {
    match expr {
        syn::Expr::Call(c) => {
            if let syn::Expr::Path(p) = &*c.func {
                if let Some(segment) = p.path.segments.last() {
                    let ident = segment.ident.to_string();
                    if ident == "require_auth" || ident == "require_auth_for_args" {
                        summary.has_auth = true;
                    }
                }
            }
            for arg in &c.args {
                check_expr(arg, summary);
            }
        }
        syn::Expr::MethodCall(m) => {
            let method_name = m.method.to_string();
            if method_name == "set"
                || method_name == "update"
                || method_name == "remove"
                || method_name == "extend_ttl"
            // Soroban v21: TTL extension counts as storage mutation
            {
                let receiver_str = quote::quote!(#m.receiver).to_string();
                if receiver_str.contains("storage")
                    || receiver_str.contains("persistent")
                    || receiver_str.contains("temporary")
                    || receiver_str.contains("instance")
                {
                    summary.has_mutation = true;
                }
            }
            if method_name == "require_auth" || method_name == "require_auth_for_args" {
                summary.has_auth = true;
            }
            if is_external_contract_method_call(m) {
                summary.has_external_call = true;
            }
            check_expr(&m.receiver, summary);
            for arg in &m.args {
                check_expr(arg, summary);
            }
        }
        syn::Expr::Block(b) => check_fn_body(&b.block, summary),
        syn::Expr::If(i) => {
            check_expr(&i.cond, summary);
            check_fn_body(&i.then_branch, summary);
            if let Some((_, else_expr)) = &i.else_branch {
                check_expr(else_expr, summary);
            }
        }
        syn::Expr::Match(m) => {
            check_expr(&m.expr, summary);
            for arm in &m.arms {
                check_expr(&arm.body, summary);
            }
        }
        _ => {}
    }
}

fn is_external_contract_method_call(method_call: &syn::ExprMethodCall) -> bool {
    if method_call.method == "invoke_contract" {
        return true;
    }

    receiver_looks_like_external_client(&method_call.receiver)
        && !method_looks_read_only(&method_call.method.to_string())
}

fn receiver_looks_like_external_client(expr: &syn::Expr) -> bool {
    match expr {
        syn::Expr::Call(call) => {
            if let syn::Expr::Path(path) = &*call.func {
                return path_looks_like_client_constructor(&path.path);
            }
            false
        }
        syn::Expr::Path(path) => path
            .path
            .segments
            .last()
            .map(|segment| ident_looks_like_client(&segment.ident.to_string()))
            .unwrap_or(false),
        syn::Expr::Reference(reference) => receiver_looks_like_external_client(&reference.expr),
        syn::Expr::Paren(paren) => receiver_looks_like_external_client(&paren.expr),
        syn::Expr::Group(group) => receiver_looks_like_external_client(&group.expr),
        _ => false,
    }
}

fn path_looks_like_client_constructor(path: &syn::Path) -> bool {
    let mut saw_client_type = false;

    for segment in &path.segments {
        let ident = segment.ident.to_string();
        if ident_looks_like_client(&ident) {
            saw_client_type = true;
        }

        if ident == "new" && saw_client_type {
            return true;
        }
    }

    false
}

fn ident_looks_like_client(ident: &str) -> bool {
    let lower = ident.to_lowercase();
    lower.ends_with("client") || lower.ends_with("_client")
}

fn method_looks_read_only(method_name: &str) -> bool {
    matches!(
        method_name,
        "balance" | "paused" | "allowance" | "decimals" | "name" | "symbol"
    ) || method_name.starts_with("get_")
        || method_name.starts_with("is_")
        || method_name.starts_with("has_")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flags_public_fn_with_storage_mutation_and_no_auth() {
        let rule = AuthGapRule::new();
        let source = r#"
            impl MyContract {
                pub fn set_admin(env: Env, new_admin: Address) {
                    env.storage().persistent().set(&symbol_short!("admin"), &new_admin);
                }
            }
        "#;
        let violations = rule.check(source);
        assert!(!violations.is_empty(), "missing auth should be flagged");
        assert!(violations[0].message.contains("set_admin"));
    }

    #[test]
    fn no_violation_when_require_auth_present() {
        let rule = AuthGapRule::new();
        let source = r#"
            impl MyContract {
                pub fn set_admin(env: Env, new_admin: Address) {
                    new_admin.require_auth();
                    env.storage().persistent().set(&symbol_short!("admin"), &new_admin);
                }
            }
        "#;
        let violations = rule.check(source);
        assert!(
            violations.is_empty(),
            "function with require_auth must not be flagged"
        );
    }

    #[test]
    fn empty_source_produces_no_findings() {
        let rule = AuthGapRule::new();
        let violations = rule.check("");
        assert!(
            violations.is_empty(),
            "empty source must produce no findings"
        );
    }

    #[test]
    fn reserved_entrypoint_constructor_not_flagged() {
        let rule = AuthGapRule::new();
        let source = r#"
            impl MyContract {
                pub fn __constructor(env: Env, admin: Address) {
                    env.storage().instance().set(&symbol_short!("admin"), &admin);
                }
            }
        "#;
        let violations = rule.check(source);
        assert!(violations.is_empty(), "__constructor must not be flagged");
    }

    #[test]
    fn private_function_with_storage_mutation_not_flagged() {
        let rule = AuthGapRule::new();
        let source = r#"
            impl MyContract {
                fn internal_set(env: &Env, key: Symbol, val: u32) {
                    env.storage().persistent().set(&key, &val);
                }
            }
        "#;
        let violations = rule.check(source);
        assert!(
            violations.is_empty(),
            "private functions must not be flagged"
        );
    }

    #[test]
    fn invalid_source_produces_no_panic() {
        let rule = AuthGapRule::new();
        let violations = rule.check("not valid rust {{{{");
        assert!(
            violations.is_empty(),
            "parse error must return empty, not panic"
        );
    }

    // ── Input validation guards ───────────────────────────────────────────────

    #[test]
    fn null_byte_source_produces_error_violation() {
        let rule = AuthGapRule::new();
        let source = "fn foo() { \0 }";
        let violations = rule.check(source);
        assert_eq!(
            violations.len(),
            1,
            "null-byte input must emit exactly one violation"
        );
        assert_eq!(violations[0].severity, super::Severity::Error);
        assert!(
            violations[0].message.contains("null bytes"),
            "message must mention null bytes; got: {}",
            violations[0].message
        );
        assert_eq!(violations[0].location, "<source>");
    }

    #[test]
    fn oversized_source_produces_error_violation() {
        use crate::input_validation::MAX_SOURCE_BYTES;
        let rule = AuthGapRule::new();
        let over = "x".repeat(MAX_SOURCE_BYTES + 1);
        let violations = rule.check(&over);
        assert_eq!(
            violations.len(),
            1,
            "oversized input must emit exactly one violation"
        );
        assert_eq!(violations[0].severity, super::Severity::Error);
        assert!(
            violations[0].message.contains("too large")
                || violations[0].message.contains("maximum"),
            "message must mention size limit; got: {}",
            violations[0].message
        );
        assert_eq!(violations[0].location, "<source>");
    }

    #[test]
    fn violation_location_includes_line_number() {
        let rule = AuthGapRule::new();
        let source = r#"
            impl MyContract {
                pub fn set_value(env: Env, v: u32) {
                    env.storage().persistent().set(&symbol_short!("V"), &v);
                }
            }
        "#;
        let violations = rule.check(source);
        assert!(!violations.is_empty(), "must detect auth gap");
        let loc = &violations[0].location;
        assert!(
            loc.contains(':'),
            "location must be 'fn_name:line', got: {loc}"
        );
        let parts: Vec<&str> = loc.splitn(2, ':').collect();
        assert_eq!(parts[0], "set_value");
        let line: usize = parts[1].parse().expect("line part must be a number");
        assert!(line > 0, "line number must be positive");
    }

    #[test]
    fn whitespace_only_source_produces_no_findings() {
        let rule = AuthGapRule::new();
        // Whitespace is non-empty (passes size guard) but parses to an empty AST.
        let violations = rule.check("   \n\t  ");
        assert!(
            violations.is_empty(),
            "whitespace-only source must produce no findings"
        );
    }

    #[test]
    fn crlf_source_produces_same_findings_as_lf() {
        let rule = AuthGapRule::new();
        let lf = r#"
impl MyContract {
    pub fn set_admin(env: Env, admin: Address) {
        env.storage().persistent().set(&symbol_short!("A"), &admin);
    }
}
"#;
        let crlf = lf.replace('\n', "\r\n");
        let lf_violations = rule.check(lf);
        let crlf_violations = rule.check(&crlf);
        assert_eq!(
            lf_violations.len(),
            crlf_violations.len(),
            "violation count must be identical for LF and CRLF input"
        );
    }
}
