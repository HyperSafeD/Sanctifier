//! Constant folding for arithmetic expressions.
//!
//! Rules such as [`crate::rules::arithmetic_overflow`] flag unchecked arithmetic
//! operators on the assumption that at least one operand is a runtime value
//! whose range is unknown to the analyzer. When every operand in an expression
//! is a literal (e.g. `1000 + 500`), the result is a compile-time constant that
//! `rustc` itself evaluates and bounds-checks — flagging it as an overflow risk
//! is just noise. [`fold_to_i128`] recursively evaluates such expressions so
//! callers can skip emitting a finding when the whole expression folds.

/// Attempts to evaluate `expr` as a compile-time integer constant.
///
/// Returns `Some(value)` if every leaf in the expression tree is an integer
/// literal (optionally negated, parenthesized, or cast), and the expression is
/// built entirely from `+ - * / %`. Returns `None` as soon as any operand is
/// not statically known (a variable, function call, field access, etc.), since
/// the rule should still treat that expression as carrying runtime risk.
pub fn fold_to_i128(expr: &syn::Expr) -> Option<i128> {
    match expr {
        syn::Expr::Lit(syn::ExprLit {
            lit: syn::Lit::Int(int),
            ..
        }) => int.base10_parse::<i128>().ok(),
        syn::Expr::Unary(syn::ExprUnary {
            op: syn::UnOp::Neg(_),
            expr,
            ..
        }) => fold_to_i128(expr).and_then(i128::checked_neg),
        syn::Expr::Paren(syn::ExprParen { expr, .. }) => fold_to_i128(expr),
        syn::Expr::Group(syn::ExprGroup { expr, .. }) => fold_to_i128(expr),
        syn::Expr::Cast(syn::ExprCast { expr, .. }) => fold_to_i128(expr),
        syn::Expr::Binary(syn::ExprBinary {
            left, op, right, ..
        }) => {
            let l = fold_to_i128(left)?;
            let r = fold_to_i128(right)?;
            match op {
                syn::BinOp::Add(_) => l.checked_add(r),
                syn::BinOp::Sub(_) => l.checked_sub(r),
                syn::BinOp::Mul(_) => l.checked_mul(r),
                syn::BinOp::Div(_) => l.checked_div(r),
                syn::BinOp::Rem(_) => l.checked_rem(r),
                _ => None,
            }
        }
        _ => None,
    }
}

/// Returns `true` if `expr` is entirely made up of compile-time integer
/// constants, i.e. [`fold_to_i128`] succeeds.
pub fn is_foldable_constant(expr: &syn::Expr) -> bool {
    fold_to_i128(expr).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_str;

    fn fold(src: &str) -> Option<i128> {
        let expr: syn::Expr = parse_str(src).unwrap();
        fold_to_i128(&expr)
    }

    #[test]
    fn folds_simple_addition() {
        assert_eq!(fold("1000 + 500"), Some(1500));
    }

    #[test]
    fn folds_nested_arithmetic() {
        assert_eq!(fold("(1 + 2) * 3 - 4"), Some(5));
    }

    #[test]
    fn folds_negative_literal() {
        assert_eq!(fold("-5 + 10"), Some(5));
    }

    #[test]
    fn folds_through_cast() {
        assert_eq!(fold("(1 + 2) as i128"), Some(3));
    }

    #[test]
    fn returns_none_for_variable_operand() {
        assert_eq!(fold("a + 2"), None);
    }

    #[test]
    fn returns_none_for_function_call_operand() {
        assert_eq!(fold("foo() + 2"), None);
    }

    #[test]
    fn returns_none_on_overflow() {
        assert_eq!(fold("170141183460469231731687303715884105727 + 1"), None);
    }

    #[test]
    fn returns_none_on_division_by_zero() {
        assert_eq!(fold("1 / 0"), None);
    }

    #[test]
    fn is_foldable_constant_matches_fold_result() {
        assert!(is_foldable_constant(
            &parse_str::<syn::Expr>("1000 + 500").unwrap()
        ));
        assert!(!is_foldable_constant(
            &parse_str::<syn::Expr>("a + 500").unwrap()
        ));
    }
}
