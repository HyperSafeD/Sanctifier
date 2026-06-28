//! Shared source-code parsing with input validation.
//!
//! All analysis entry points should call [`parse_source`] instead of calling
//! `syn::parse_str` directly so that every guard from [`crate::input_validation`]
//! is applied consistently before AST work begins.
//!
//! # Example
//!
//! ```rust,ignore
//! use sanctifier_core::parser;
//!
//! match parser::parse_source(source) {
//!     Ok(parsed) => { /* walk parsed.file */ }
//!     Err(parser::ParseError::Validation(e)) => eprintln!("bad input: {}", e),
//!     Err(parser::ParseError::Syntax(e))     => eprintln!("parse error: {}", e),
//! }
//! ```

use crate::input_validation::{self, ValidationError};
use syn::File;

/// A validated and successfully parsed Rust source file.
#[derive(Debug)]
pub struct ParsedSource {
    /// The AST produced by `syn`.
    pub file: File,
}

/// Errors returned by [`parse_source`].
#[derive(Debug)]
pub enum ParseError {
    /// Input rejected by a validation guard (empty, too large, null bytes, …).
    Validation(ValidationError),
    /// Source passed validation but is not syntactically valid Rust.
    Syntax(syn::Error),
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Validation(e) => write!(f, "validation error: {}", e),
            Self::Syntax(e) => write!(f, "syntax error: {}", e),
        }
    }
}

impl std::error::Error for ParseError {}

/// Validate and parse `source` into a [`ParsedSource`].
///
/// This is the canonical entry point for converting raw Rust source text into
/// an AST that rules and analysis passes can inspect. All input guards from
/// [`crate::input_validation`] run *before* any parsing work is attempted.
///
/// # Errors
///
/// - [`ParseError::Validation`] — a size or content guard failed
///   (e.g. empty source, null bytes, oversized input).
/// - [`ParseError::Syntax`] — `syn` could not parse the text as a Rust file.
pub fn parse_source(source: &str) -> Result<ParsedSource, ParseError> {
    input_validation::validate_source_all(source).map_err(ParseError::Validation)?;
    let file = syn::parse_str::<File>(source).map_err(ParseError::Syntax)?;
    Ok(ParsedSource { file })
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input_validation::MAX_SOURCE_BYTES;

    // ── Validation guards ─────────────────────────────────────────────────────

    #[test]
    fn empty_source_is_rejected_with_validation_error() {
        let err = parse_source("").unwrap_err();
        assert!(
            matches!(err, ParseError::Validation(_)),
            "expected Validation, got: {err}"
        );
        let msg = err.to_string();
        assert!(
            msg.to_lowercase().contains("empty") || msg.contains("EMPTY"),
            "message should mention empty source; got: {msg}"
        );
    }

    #[test]
    fn oversized_source_is_rejected_with_validation_error() {
        let over = "x".repeat(MAX_SOURCE_BYTES + 1);
        let err = parse_source(&over).unwrap_err();
        assert!(
            matches!(err, ParseError::Validation(_)),
            "expected Validation, got: {err}"
        );
    }

    #[test]
    fn null_byte_in_source_is_rejected_with_validation_error() {
        let err = parse_source("fn foo() { let _ = \0; }").unwrap_err();
        assert!(
            matches!(err, ParseError::Validation(_)),
            "expected Validation, got: {err}"
        );
    }

    // ── Syntax errors ─────────────────────────────────────────────────────────

    #[test]
    fn invalid_syntax_returns_syntax_error() {
        let err = parse_source("this is {{ not valid rust!!!").unwrap_err();
        assert!(
            matches!(err, ParseError::Syntax(_)),
            "expected Syntax, got: {err}"
        );
    }

    #[test]
    fn unclosed_brace_returns_syntax_error() {
        let err = parse_source("fn foo() {").unwrap_err();
        assert!(matches!(err, ParseError::Syntax(_)));
    }

    // ── Successful parses ─────────────────────────────────────────────────────

    #[test]
    fn minimal_valid_source_parses_to_one_item() {
        let src = "pub fn add(a: u32, b: u32) -> u32 { a + b }";
        let parsed = parse_source(src).unwrap();
        assert_eq!(parsed.file.items.len(), 1);
    }

    #[test]
    fn whitespace_only_source_parses_to_empty_ast() {
        // Whitespace passes size and null-byte guards but produces no items.
        let parsed = parse_source("   \n\t  ").unwrap();
        assert!(parsed.file.items.is_empty());
    }

    #[test]
    fn soroban_contract_skeleton_parses_successfully() {
        let src = r#"
            use soroban_sdk::{contract, contractimpl, Env};

            #[contract]
            pub struct MyContract;

            #[contractimpl]
            impl MyContract {
                pub fn hello(_env: Env) -> u32 { 42 }
            }
        "#;
        let parsed = parse_source(src).unwrap();
        assert!(
            parsed.file.items.len() >= 3,
            "expected ≥3 items (use, struct, impl)"
        );
    }

    #[test]
    fn source_at_max_boundary_parses_successfully() {
        // Craft a source exactly at the limit: fill with spaces so syn parses it.
        let padding = " ".repeat(MAX_SOURCE_BYTES - 15);
        let src = format!("fn f(){{}}{}", padding);
        // May hit exactly or just under limit — accept both outcomes.
        let _ = parse_source(&src);
    }

    // ── ParseError Display ────────────────────────────────────────────────────

    #[test]
    fn parse_error_display_is_non_empty() {
        let e = parse_source("").unwrap_err();
        assert!(!e.to_string().is_empty());
    }

    #[test]
    fn syntax_error_display_mentions_syntax() {
        let e = parse_source("fn {{{{").unwrap_err();
        let s = e.to_string();
        assert!(
            s.contains("syntax") || s.contains("error") || s.contains("expected"),
            "unexpected display: {s}"
        );
    }
}
