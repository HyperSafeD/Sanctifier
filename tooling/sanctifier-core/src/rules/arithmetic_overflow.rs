use crate::rules::{Rule, RuleViolation, Severity};
use crate::ArithmeticIssue;
use rayon::prelude::*;
use std::collections::HashSet;
use std::sync::Mutex;
use syn::spanned::Spanned;
use syn::visit::Visit;
use syn::{parse_str, File};

/// **S003: Arithmetic Overflow / Underflow Detection Rule**
///
/// This rule detects unchecked arithmetic operations that could overflow or underflow
/// in Soroban smart contracts. Integer overflow/underflow in financial applications
/// can lead to critical vulnerabilities including loss of funds, incorrect balances,
/// and unauthorized minting.
///
/// # Detection Scope
///
/// The rule flags:
/// - Binary operators: `+`, `-`, `*`, `/`, `%`
/// - Compound assignments: `+=`, `-=`, `*=`, `/=`, `%=`
/// - Custom math methods: `.mul_div()`, `.fixed_point_mul()`, `.fixed_point_div()`, `.div_ceil()`
/// - Custom math functions: `mul_div()`, `fixed_point_mul()`, `fixed_point_div()`
///
/// # Exclusions
///
/// The rule does NOT flag:
/// - Test code (`#[test]` functions and `#[cfg(test)]` modules)
/// - Array/slice indexing arithmetic (e.g., `buf[i + 1]`)
/// - Comparison operators (`>`, `<`, `>=`, `<=`, `==`, `!=`)
/// - Bitwise operators (`&`, `|`, `^`, `<<`, `>>`)
/// - String concatenation
/// - Safe methods (`.checked_*()`, `.saturating_*()`)
///
/// # Deduplication
///
/// To reduce noise, the rule reports **at most one finding per (function_name, operation) pair**.
/// If a function uses `+` multiple times, only one S003 finding is reported for that function.
///
/// # Output
///
/// Each finding includes:
/// - `function_name`: The function where the operation occurs
/// - `operation`: The operator or method (e.g., `"+"`, `"mul_div"`)
/// - `suggestion`: Remediation guidance (e.g., "Use .checked_add(rhs)...")
/// - `location`: Function name and line number (e.g., "transfer:42")
///
/// # Example
///
/// ```rust,ignore
/// pub fn mint(env: Env, to: Address, amount: i128) {
///     let balance = get_balance(&env, &to);
///     let new_balance = balance + amount;  // S003: flagged
///     set_balance(&env, &to, new_balance);
/// }
///
/// // Safe alternative:
/// pub fn mint_safe(env: Env, to: Address, amount: i128) {
///     let balance = get_balance(&env, &to);
///     let new_balance = balance.checked_add(amount)  // Not flagged
///         .expect("mint: overflow");
///     set_balance(&env, &to, new_balance);
/// }
/// ```
///
/// # References
///
/// - [S003 Documentation](https://github.com/HyperSafeD/Sanctifier/blob/main/docs/rules/s003-arithmetic-overflow.md)
/// - [Finding Code Reference](https://github.com/HyperSafeD/Sanctifier/blob/main/docs/error-codes.md#s003)
pub struct ArithmeticOverflowRule;

impl ArithmeticOverflowRule {
    /// Create a new instance of the arithmetic overflow rule.
    pub fn new() -> Self {
        Self
    }
}

impl Default for ArithmeticOverflowRule {
    fn default() -> Self {
        Self::new()
    }
}

impl Rule for ArithmeticOverflowRule {
    fn name(&self) -> &str {
        "arithmetic_overflow"
    }

    fn description(&self) -> &str {
        "Detects unchecked arithmetic operations that could overflow or underflow"
    }

    fn check(&self, source: &str) -> Vec<RuleViolation> {
        let file = match parse_str::<File>(source) {
            Ok(f) => f,
            Err(_) => return vec![],
        };

        let mut visitor = ArithVisitor {
            issues: Vec::new(),
            current_fn: None,
            seen: HashSet::new(),
            index_depth: 0,
            test_mod_depth: 0,
        };
        visitor.visit_file(&file);

        visitor
            .issues
            .into_iter()
            .map(|issue| {
                RuleViolation::new(
                    self.name(),
                    Severity::Warning,
                    format!("Unchecked '{}' operation could overflow", issue.operation),
                    issue.location,
                )
                .with_suggestion(issue.suggestion)
            })
            .collect()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl ArithmeticOverflowRule {
    /// Check multiple source files in parallel using rayon.
    ///
    /// This method enables concurrent analysis of multiple files or AST nodes,
    /// significantly improving performance for large monorepos.
    ///
    /// # Performance
    ///
    /// - **Sequential**: O(n * m) where n = files, m = avg file size
    /// - **Parallel**: O(m) with n cores (linear speedup)
    ///
    /// # Memory
    ///
    /// Uses thread-safe Mutex for collecting violations across threads.
    /// Memory overhead: ~64 bytes per thread + violations.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use sanctifier_core::rules::arithmetic_overflow::ArithmeticOverflowRule;
    /// 
    /// let rule = ArithmeticOverflowRule::new();
    /// let sources = vec!["contract1.rs", "contract2.rs"];
    /// let violations = rule.check_parallel(&sources);
    /// ```
    pub fn check_parallel(&self, sources: &[&str]) -> Vec<RuleViolation> {
        let violations = Mutex::new(Vec::new());
        
        sources.par_iter().for_each(|source| {
            let file_violations = self.check(source);
            if !file_violations.is_empty() {
                violations.lock().unwrap().extend(file_violations);
            }
        });
        
        violations.into_inner().unwrap()
    }
    
    /// Check multiple files from paths in parallel.
    ///
    /// Reads files concurrently and analyzes them in parallel for
    /// maximum throughput on multi-core systems.
    ///
    /// # Arguments
    ///
    /// - `paths`: Slice of file paths to analyze
    ///
    /// # Returns
    ///
    /// Combined violations from all files, with file paths included
    /// in the location string.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use sanctifier_core::rules::arithmetic_overflow::ArithmeticOverflowRule;
    /// use std::path::Path;
    /// 
    /// let rule = ArithmeticOverflowRule::new();
    /// let paths: Vec<&Path> = vec![Path::new("src/lib.rs"), Path::new("src/main.rs")];
    /// let violations = rule.check_files_parallel(&paths);
    /// ```
    pub fn check_files_parallel(&self, paths: &[&std::path::Path]) -> Vec<RuleViolation> {
        let violations = Mutex::new(Vec::new());
        
        paths.par_iter().for_each(|path| {
            if let Ok(source) = std::fs::read_to_string(path) {
                let mut file_violations = self.check(&source);
                
                // Add file path to location for better error reporting
                let file_name = path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown");
                    
                for violation in &mut file_violations {
                    violation.location = format!("{}:{}", file_name, violation.location);
                }
                
                if !file_violations.is_empty() {
                    violations.lock().unwrap().extend(file_violations);
                }
            }
        });
        
        violations.into_inner().unwrap()
    }
}

pub(crate) struct ArithVisitor {
    /// Issues found during AST traversal.
    pub(crate) issues: Vec<ArithmeticIssue>,
    /// Current function name context (None when outside any function).
    pub(crate) current_fn: Option<String>,
    /// Deduplication set: (function_name, operation) pairs already reported.
    /// Prevents multiple findings for the same operator in one function.
    pub(crate) seen: HashSet<(String, String)>,
    /// Depth counter for array index expressions.
    /// When >0, we are inside an index subscript and skip arithmetic detection.
    /// This prevents flagging idiomatic patterns like `buf[i + 1]`.
    pub(crate) index_depth: u32,
    /// Depth counter for #[cfg(test)] modules.
    /// When >0, we skip all arithmetic detection to avoid false positives in tests.
    pub(crate) test_mod_depth: u32,
}

// Redundant ArithmeticIssue struct removed

impl ArithVisitor {
    /// Checks if an expression is a compile-time constant.
    ///
    /// Returns `true` for:
    /// - Literal values: `42`, `true`, `"string"`
    /// - Negated literals: `-5`
    /// - ALL_CAPS identifiers (CONSTANT naming convention)
    /// - Parenthesized/cast constants
    ///
    /// # Note
    ///
    /// This is a heuristic and may have false positives/negatives.
    /// Used primarily for potential future optimization to skip constant-folded expressions.
    #[allow(dead_code)]
    fn is_constant_expr(expr: &syn::Expr) -> bool {
        match expr {
            syn::Expr::Lit(_) => true,
            syn::Expr::Unary(syn::ExprUnary {
                op: syn::UnOp::Neg(_),
                expr,
                ..
            }) => {
                matches!(expr.as_ref(), syn::Expr::Lit(_))
            }
            syn::Expr::Paren(syn::ExprParen { expr, .. }) => Self::is_constant_expr(expr),
            syn::Expr::Cast(syn::ExprCast { expr, .. }) => Self::is_constant_expr(expr),
            syn::Expr::Path(path) => {
                if let Some(seg) = path.path.segments.last() {
                    let name = seg.ident.to_string();
                    name.chars()
                        .all(|c| c.is_uppercase() || c == '_' || c.is_ascii_digit())
                        && !name.is_empty()
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    /// Checks if a binary operation is division or modulo with a non-constant divisor.
    ///
    /// Returns `true` for:
    /// - `a / variable`
    /// - `a % variable`
    ///
    /// These are particularly dangerous because they can panic at runtime
    /// (division by zero / modulo by zero) rather than just overflow.
    ///
    /// # Note
    ///
    /// Currently unused but available for future severity escalation logic.
    #[allow(dead_code)]
    fn is_non_constant_divisor(op: &syn::BinOp, right: &syn::Expr) -> bool {
        matches!(op, syn::BinOp::Div(_) | syn::BinOp::Rem(_)) && !Self::is_constant_expr(right)
    }

    /// Classifies a binary operator and returns (operator_string, suggestion) if it's risky.
    ///
    /// # Returns
    ///
    /// - `Some((op_str, suggestion))` for operators that need checking
    /// - `None` for safe operators (comparison, bitwise, logical)
    ///
    /// # Operator Categories
    ///
    /// **Arithmetic (flagged):**
    /// - `+`, `-`, `*`, `/`, `%`
    /// - `+=`, `-=`, `*=`, `/=`, `%=`
    ///
    /// **Comparison (not flagged):**
    /// - `<`, `>`, `<=`, `>=`, `==`, `!=`
    ///
    /// **Bitwise (not flagged):**
    /// - `&`, `|`, `^`, `<<`, `>>`
    ///
    /// **Logical (not flagged):**
    /// - `&&`, `||`
    fn classify_op(op: &syn::BinOp) -> Option<(&'static str, &'static str)> {
        match op {
            syn::BinOp::Add(_) => Some((
                "+",
                "Use .checked_add(rhs) or .saturating_add(rhs) to handle overflow",
            )),
            syn::BinOp::Sub(_) => Some((
                "-",
                "Use .checked_sub(rhs) or .saturating_sub(rhs) to handle underflow",
            )),
            syn::BinOp::Mul(_) => Some((
                "*",
                "Use .checked_mul(rhs) or .saturating_mul(rhs) to handle overflow",
            )),
            syn::BinOp::Div(_) => {
                Some(("/", "Use .checked_div(rhs) to avoid division-by-zero panic"))
            }
            syn::BinOp::Rem(_) => {
                Some(("%", "Use .checked_rem(rhs) to avoid modulo-by-zero panic"))
            }
            syn::BinOp::AddAssign(_) => Some((
                "+=",
                "Replace a += b with a = a.checked_add(b).expect(\"overflow\")",
            )),
            syn::BinOp::SubAssign(_) => Some((
                "-=",
                "Replace a -= b with a = a.checked_sub(b).expect(\"underflow\")",
            )),
            syn::BinOp::MulAssign(_) => Some((
                "*=",
                "Replace a *= b with a = a.checked_mul(b).expect(\"overflow\")",
            )),

            syn::BinOp::DivAssign(_) => Some((
                "/=",
                "Replace a /= b with a = a.checked_div(b).expect(\"division by zero\")",
            )),
            syn::BinOp::RemAssign(_) => Some((
                "%=",
                "Replace a %= b with a = a.checked_rem(b).expect(\"modulo by zero\")",
            )),
            _ => None,
        }
    }
}

impl<'ast> Visit<'ast> for ArithVisitor {
    // ── Module-level: skip #[cfg(test)] modules entirely ─────────────────────

    /// Visit item module - tracks entry/exit from `#[cfg(test)]` modules.
    ///
    /// When inside a test module, `test_mod_depth > 0` and all arithmetic
    /// detection is suppressed to avoid false positives in test code.
    fn visit_item_mod(&mut self, node: &'ast syn::ItemMod) {
        if is_cfg_test(&node.attrs) {
            self.test_mod_depth += 1;
            syn::visit::visit_item_mod(self, node);
            self.test_mod_depth -= 1;
        } else {
            syn::visit::visit_item_mod(self, node);
        }
    }

    /// Visit impl item function - tracks current function context for findings.
    ///
    /// Skips functions with `#[test]` attribute or inside test modules.
    /// Sets `current_fn` for location tracking in findings.
    fn visit_impl_item_fn(&mut self, node: &'ast syn::ImplItemFn) {
        if self.test_mod_depth > 0 || has_test_attr(&node.attrs) {
            return;
        }
        let prev = self.current_fn.take();
        self.current_fn = Some(node.sig.ident.to_string());
        syn::visit::visit_impl_item_fn(self, node);
        self.current_fn = prev;
    }

    /// Visit standalone function - tracks current function context.
    ///
    /// Skips test functions and functions inside test modules.
    fn visit_item_fn(&mut self, node: &'ast syn::ItemFn) {
        if self.test_mod_depth > 0 || has_test_attr(&node.attrs) {
            return;
        }
        let prev = self.current_fn.take();
        self.current_fn = Some(node.sig.ident.to_string());
        syn::visit::visit_item_fn(self, node);
        self.current_fn = prev;
    }

    // ── Index expressions: don't flag arithmetic in subscripts ────────────────

    /// Visit index expression - suppresses arithmetic detection inside array subscripts.
    ///
    /// Pattern like `buf[i + 1]` is idiomatic Rust and should not be flagged.
    /// We visit the array expression normally, but increase `index_depth` before
    /// visiting the index to suppress arithmetic findings.
    fn visit_expr_index(&mut self, node: &'ast syn::ExprIndex) {
        // Visit the object expression normally (it may contain calls, etc.)
        self.visit_expr(&node.expr);
        // Increase depth so arithmetic inside the index is suppressed.
        self.index_depth += 1;
        self.visit_expr(&node.index);
        self.index_depth -= 1;
    }

    /// Visit binary expression - detects unchecked arithmetic operators.
    ///
    /// Core detection logic for S003. Checks if:
    /// 1. We're not inside an array index (`index_depth == 0`)
    /// 2. We're inside a function (`current_fn.is_some()`)
    /// 3. The operator is arithmetic (`classify_op()` returns `Some`)
    /// 4. Neither operand is a string literal
    /// 5. We haven't already reported this (function, operator) pair
    ///
    /// If all conditions are met, creates an `ArithmeticIssue` finding.
    fn visit_expr_binary(&mut self, node: &'ast syn::ExprBinary) {
        if self.index_depth == 0 {
            if let Some(fn_name) = self.current_fn.clone() {
                if let Some((op_str, suggestion)) = Self::classify_op(&node.op) {
                    if !is_string_literal(&node.left)
                        && !is_string_literal(&node.right)
                        && !crate::constant_folding::is_foldable_constant(&syn::Expr::Binary(
                            node.clone(),
                        ))
                    {
                        let key = (fn_name.clone(), op_str.to_string());
                        if !self.seen.contains(&key) {
                            self.seen.insert(key);
                            let line = node.left.span().start().line;
                            self.issues.push(ArithmeticIssue {
                                function_name: fn_name.clone(),
                                operation: op_str.to_string(),
                                suggestion: suggestion.to_string(),
                                location: format!("{}:{}", fn_name, line),
                            });
                        }
                    }
                }
            }
        }
        syn::visit::visit_expr_binary(self, node);
    }

    /// Visit method call expression - detects unchecked custom math methods.
    ///
    /// Detects risky method patterns like:
    /// - `.mul_div(numerator, denominator)` - can overflow before division
    /// - `.div_ceil(divisor)` - potential boundary issues
    /// - `.fixed_point_mul(factor)` - fixed-point math without overflow checks
    /// - `.fixed_point_div(divisor)` - fixed-point division
    ///
    /// Safe variants (not flagged):
    /// - `.checked_mul_div(...)`
    /// - `.checked_fixed_point_mul(...)`
    /// - `.checked_fixed_point_div(...)`
    fn visit_expr_method_call(&mut self, node: &'ast syn::ExprMethodCall) {
        if let Some(fn_name) = self.current_fn.clone() {
            let method_name = node.method.to_string();
            if let Some(suggestion) = classify_math_method(&method_name) {
                let key = (fn_name.clone(), method_name.clone());
                if !self.seen.contains(&key) {
                    self.seen.insert(key);
                    let line = node.span().start().line;
                    self.issues.push(ArithmeticIssue {
                        function_name: fn_name.clone(),
                        operation: method_name,
                        suggestion,
                        location: format!("{}:{}", fn_name, line),
                    });
                }
            }
        }
        syn::visit::visit_expr_method_call(self, node);
    }

    /// Visit function call expression - detects unchecked custom math functions.
    ///
    /// Detects risky function-style math operations:
    /// - `mul_div(a, b, c)` - multiplication-division combo
    /// - `fixed_point_mul(a, b)` - fixed-point multiplication
    /// - `fixed_point_div(a, b)` - fixed-point division
    ///
    /// These are typically utility functions that may not have overflow protection.
    fn visit_expr_call(&mut self, node: &'ast syn::ExprCall) {
        if let Some(fn_name) = self.current_fn.clone() {
            if let syn::Expr::Path(expr_path) = &*node.func {
                if let Some(last_segment) = expr_path.path.segments.last() {
                    let func_name = last_segment.ident.to_string();
                    if let Some(suggestion) = classify_math_call(&func_name) {
                        let key = (fn_name.clone(), func_name.clone());
                        if !self.seen.contains(&key) {
                            self.seen.insert(key);
                            let line = node.span().start().line;
                            self.issues.push(ArithmeticIssue {
                                function_name: fn_name.clone(),
                                operation: func_name,
                                suggestion,
                                location: format!("{}:{}", fn_name, line),
                            });
                        }
                    }
                }
            }
        }
        syn::visit::visit_expr_call(self, node);
    }
}

/// Classifies custom math method calls that lack overflow protection.
///
/// # Detected Patterns
///
/// - `mul_div(a, b)` - Multiply then divide, can overflow in intermediate multiplication
/// - `div_ceil(d)` - Ceiling division, may have boundary issues
/// - `fixed_point_mul(f)` - Fixed-point multiplication without overflow checks
/// - `fixed_point_div(d)` - Fixed-point division without overflow checks
///
/// # Safe Alternatives
///
/// Methods starting with `checked_` are considered safe and not flagged:
/// - `checked_mul_div()`
/// - `checked_fixed_point_mul()`
/// - `checked_fixed_point_div()`
///
/// # Returns
///
/// - `Some(suggestion)` if the method is risky
/// - `None` if the method is safe or not recognized
fn classify_math_method(method: &str) -> Option<String> {
    match method {
        "mul_div" => Some("Use '.checked_mul_div()' to handle potential overflow".to_string()),
        "div_ceil" => {
            Some("Consider '.checked_div()' if boundary verification is required".to_string())
        }
        "fixed_point_mul" => Some("Use '.checked_fixed_point_mul()' for safety".to_string()),
        "fixed_point_div" => Some("Use '.checked_fixed_point_div()' for safety".to_string()),
        _ => None,
    }
}

/// Classifies custom math function calls that lack overflow protection.
///
/// Similar to `classify_math_method()` but for function-style calls rather than methods.
///
/// # Detected Patterns
///
/// - `mul_div(a, b, c)` - Function-style multiply-divide
/// - `fixed_point_mul(a, b)` - Function-style fixed-point multiply
/// - `fixed_point_div(a, b)` - Function-style fixed-point divide
///
/// # Returns
///
/// - `Some(suggestion)` if the function is risky
/// - `None` if the function is safe or not recognized
fn classify_math_call(func: &str) -> Option<String> {
    match func {
        "mul_div" => Some("Use 'checked_mul_div' to handle potential overflow".to_string()),
        "fixed_point_mul" => Some("Use 'checked_fixed_point_mul' for safety".to_string()),
        "fixed_point_div" => Some("Use 'checked_fixed_point_div' for safety".to_string()),
        _ => None,
    }
}

/// Checks if an expression is a string literal.
///
/// Used to exclude string concatenation (`"hello" + "world"`) from arithmetic
/// overflow detection, as strings use `+` for concatenation, not arithmetic.
///
/// # Returns
///
/// `true` if the expression is `Expr::Lit` containing `Lit::Str`, `false` otherwise.
fn is_string_literal(expr: &syn::Expr) -> bool {
    matches!(
        expr,
        syn::Expr::Lit(syn::ExprLit {
            lit: syn::Lit::Str(_),
            ..
        })
    )
}

/// Returns true if the item has a `#[test]` attribute.
///
/// Used to skip test functions from arithmetic overflow detection.
/// Test code often uses arithmetic that would be flagged but is intentional
/// for testing edge cases.
fn has_test_attr(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|a| a.path().is_ident("test"))
}

/// Returns true if the item has a `#[cfg(test)]` attribute.
///
/// Used to skip entire test modules from arithmetic overflow detection.
/// This is a broader exclusion than `has_test_attr()` which only excludes
/// individual functions.
fn is_cfg_test(attrs: &[syn::Attribute]) -> bool {
    attrs
        .iter()
        .any(|a| a.path().is_ident("cfg") && quote::quote!(#a).to_string().contains("test"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flag_standard_arithmetic() {
        let rule = ArithmeticOverflowRule::new();
        let source = r#"
            fn transfer() {
                let a = 1;
                let b = 2;
                let c = a + b;
                let d = a - b;
                let e = a * b;
                let f = a / b;
                let g = a % b;
            }
        "#;
        let violations = rule.check(source);
        assert_eq!(violations.len(), 5);
    }

    #[test]
    fn test_flag_custom_math_methods() {
        let rule = ArithmeticOverflowRule::new();
        let source = r#"
            fn transfer() {
                let a = 1;
                let b = 2;
                let c = a.mul_div(5, 10);
                let d = a.fixed_point_mul(b);
            }
        "#;
        let violations = rule.check(source);
        assert!(violations.iter().any(|v| v.message.contains("mul_div")));
        assert!(violations
            .iter()
            .any(|v| v.message.contains("fixed_point_mul")));
    }

    #[test]
    fn test_flag_custom_math_calls() {
        let rule = ArithmeticOverflowRule::new();
        let source = r#"
            fn transfer() {
                let a = mul_div(1, 2, 3);
                let b = fixed_point_div(10, 2);
            }
        "#;
        let violations = rule.check(source);
        assert!(violations.iter().any(|v| v.message.contains("mul_div")));
        assert!(violations
            .iter()
            .any(|v| v.message.contains("fixed_point_div")));
    }

    #[test]
    fn test_ignore_checked_methods() {
        let rule = ArithmeticOverflowRule::new();
        let source = r#"
            fn transfer() {
                let a = 1;
                let b = a.checked_add(2);
                let c = a.checked_mul_div(5, 10);
            }
        "#;
        let violations = rule.check(source);
        assert_eq!(violations.len(), 0);
    }

    #[test]
    fn test_skip_test_attribute_functions() {
        let rule = ArithmeticOverflowRule::new();
        // A #[test] fn with arithmetic should produce zero violations.
        let source = r#"
            #[test]
            fn my_unit_test() {
                let a = 1u64;
                let b = 2u64;
                let c = a + b;
                let d = a - b;
                let e = a * b;
            }
        "#;
        let violations = rule.check(source);
        assert_eq!(violations.len(), 0, "#[test] fns must be skipped");
    }

    #[test]
    fn test_skip_cfg_test_module() {
        let rule = ArithmeticOverflowRule::new();
        // All arithmetic inside #[cfg(test)] mod must be ignored.
        let source = r#"
            fn mint(amount: u64) {
                let total = amount + 1;
            }

            #[cfg(test)]
            mod tests {
                fn helper() {
                    let x = 1u64 + 2u64;
                    let y = x * 10u64;
                }
            }
        "#;
        let violations = rule.check(source);
        // Only `mint` should fire (1 finding for `+`), not the cfg(test) helper.
        assert_eq!(
            violations.len(),
            1,
            "cfg(test) module arithmetic must be skipped"
        );
    }

    #[test]
    fn test_skip_index_subscript_arithmetic() {
        let rule = ArithmeticOverflowRule::new();
        // i + 1 as an array subscript is idiomatic and should not trigger.
        let source = r#"
            fn read_next(buf: &[u8], i: usize) -> u8 {
                buf[i + 1]
            }
        "#;
        let violations = rule.check(source);
        assert_eq!(
            violations.len(),
            0,
            "index subscript arithmetic must be skipped"
        );
    }

    #[test]
    fn test_skip_constant_folded_arithmetic() {
        let rule = ArithmeticOverflowRule::new();
        // 1000 + 500 is a compile-time constant; rustc evaluates and bounds
        // checks it itself, so it is not a runtime overflow risk.
        let source = r#"
            fn transfer() {
                let a = 1000 + 500;
            }
        "#;
        let violations = rule.check(source);
        assert_eq!(
            violations.len(),
            0,
            "constant-folded literal arithmetic must not be flagged"
        );
    }

    #[test]
    fn test_still_flags_when_one_operand_is_runtime() {
        let rule = ArithmeticOverflowRule::new();
        // Mixed literal + variable is still a runtime risk and must be flagged.
        let source = r#"
            fn transfer(amount: u64) {
                let a = amount + 500;
            }
        "#;
        let violations = rule.check(source);
        assert_eq!(
            violations.len(),
            1,
            "arithmetic with a non-constant operand must still be flagged"
        );
    }
}
