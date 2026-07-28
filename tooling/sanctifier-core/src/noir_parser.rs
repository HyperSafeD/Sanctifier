//! Noir (`.nr`) source parser front-end (#1228).
//!
//! Maps a subset of Noir circuit source onto the same intermediate
//! representation used by the circom front-end, so Z-rules can analyse Noir
//! circuits with minimal rule-specific special-casing.
//!
//! # Coverage
//!
//! This parser covers the subset of Noir required for Z-rule analysis:
//! - Function definitions (`fn name(params) -> ReturnType { ... }`)
//! - Public / private parameter annotations (`pub`, implicit private)
//! - `assert` and `assert_eq` constraint statements
//! - Storage reads/writes via the `Storage` struct convention
//! - Hash calls (`std::hash::pedersen_hash`, `std::hash::sha256`)
//!
//! # Gaps
//!
//! The following Noir constructs are not yet parsed and are silently ignored:
//! - Traits and trait implementations
//! - Closures
//! - Macro invocations (`comptime { ... }`)
//! - Generic type parameters beyond simple `Field` / `u64` / `bool`
//!
//! Extend `parse_statement` to cover additional constructs as Z-rule coverage
//! expands to require them.

use std::collections::HashMap;

// ── Intermediate representation ───────────────────────────────────────────────

/// Visibility of a Noir function parameter or return value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Visibility {
    /// Marked `pub` — included in the proof's public inputs.
    Public,
    /// No `pub` annotation — part of the witness (private input).
    Private,
}

/// A single parameter in a Noir function signature.
#[derive(Debug, Clone)]
pub struct NoirParam {
    pub name: String,
    pub ty: String,
    pub visibility: Visibility,
}

/// A parsed Noir function.
#[derive(Debug, Clone)]
pub struct NoirFunction {
    pub name: String,
    pub params: Vec<NoirParam>,
    pub return_type: Option<String>,
    pub return_visibility: Visibility,
    /// Flat list of statement kinds found in the body (order-preserving).
    pub body: Vec<NoirStatement>,
}

/// A simplified statement in a Noir function body.
#[derive(Debug, Clone)]
pub enum NoirStatement {
    /// `assert(expr)` or `assert(expr, "msg")`
    Assert { expr: String },
    /// `assert_eq(lhs, rhs)` or `assert_eq(lhs, rhs, "msg")`
    AssertEq { lhs: String, rhs: String },
    /// A call whose callee contains "hash" (e.g. `pedersen_hash`, `sha256`).
    HashCall { callee: String, args: Vec<String> },
    /// A storage read: `storage.field.get()`
    StorageRead { field: String },
    /// A storage write: `storage.field.write(value)` / `.insert(value)`
    StorageWrite { field: String, value: String },
    /// Any other expression statement, preserved verbatim for context.
    Other { raw: String },
}

/// The top-level IR produced by parsing a single `.nr` file.
#[derive(Debug, Default)]
pub struct NoirModule {
    /// All top-level functions defined in the file, keyed by name.
    pub functions: HashMap<String, NoirFunction>,
    /// Raw source lines that could not be attributed to any construct.
    pub unparsed: Vec<String>,
}

// ── Parser ────────────────────────────────────────────────────────────────────

/// Errors returned by [`parse_noir`].
#[derive(Debug)]
pub enum NoirParseError {
    /// The source string is empty.
    EmptySource,
    /// Source is too large to analyse safely (> 512 KiB).
    SourceTooLarge { bytes: usize },
    /// A function signature could not be parsed.
    MalformedFunction { line: usize, snippet: String },
}

impl std::fmt::Display for NoirParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptySource => write!(f, "empty Noir source"),
            Self::SourceTooLarge { bytes } => {
                write!(f, "source too large: {bytes} bytes (limit 524288)")
            }
            Self::MalformedFunction { line, snippet } => {
                write!(f, "malformed function at line {line}: {snippet:?}")
            }
        }
    }
}

impl std::error::Error for NoirParseError {}

const MAX_SOURCE_BYTES: usize = 512 * 1024;

/// Parse `source` (a `.nr` file's contents) into a [`NoirModule`].
///
/// This is a line-oriented best-effort parser. It extracts enough structure
/// for Z-rule analysis without implementing a full Noir grammar.
pub fn parse_noir(source: &str) -> Result<NoirModule, NoirParseError> {
    if source.is_empty() {
        return Err(NoirParseError::EmptySource);
    }
    if source.len() > MAX_SOURCE_BYTES {
        return Err(NoirParseError::SourceTooLarge {
            bytes: source.len(),
        });
    }

    let mut module = NoirModule::default();
    let lines: Vec<&str> = source.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let trimmed = lines[i].trim();

        if trimmed.starts_with("fn ") || trimmed.starts_with("pub fn ") {
            match parse_function(&lines, i) {
                Ok((func, consumed)) => {
                    module.functions.insert(func.name.clone(), func);
                    i += consumed;
                    continue;
                }
                Err(e) => {
                    module.unparsed.push(format!("line {i}: {e}"));
                }
            }
        } else if !trimmed.is_empty() && !trimmed.starts_with("//") {
            module.unparsed.push(trimmed.to_string());
        }

        i += 1;
    }

    Ok(module)
}

// ── Internal helpers ──────────────────────────────────────────────────────────

/// Parse a function starting at `lines[start]`. Returns `(function, lines_consumed)`.
fn parse_function(lines: &[&str], start: usize) -> Result<(NoirFunction, usize), String> {
    let header = lines[start].trim();

    let name =
        extract_fn_name(header).ok_or_else(|| format!("cannot extract name from: {header:?}"))?;

    let (params, return_type, return_visibility) = parse_signature(header);

    // Collect body lines between the opening `{` and the matching `}`.
    let mut body_lines: Vec<&str> = Vec::new();
    let mut depth = 0usize;
    let mut consumed: usize = 1;

    for line in &lines[start..] {
        for ch in line.chars() {
            match ch {
                '{' => depth += 1,
                '}' => {
                    depth = depth.saturating_sub(1);
                }
                _ => {}
            }
        }
        if consumed > 1 {
            body_lines.push(line);
        }
        consumed += 1;
        if depth == 0 && consumed > 1 {
            break;
        }
    }

    let body = body_lines
        .iter()
        .map(|l| parse_statement(l.trim()))
        .collect();

    Ok((
        NoirFunction {
            name,
            params,
            return_type,
            return_visibility,
            body,
        },
        consumed.saturating_sub(1),
    ))
}

fn extract_fn_name(header: &str) -> Option<String> {
    // Match `fn name(` or `pub fn name(`
    let after_fn = header.split("fn ").nth(1)?;
    let name = after_fn.split('(').next()?.trim().to_string();
    if name.is_empty() {
        None
    } else {
        Some(name)
    }
}

fn parse_signature(header: &str) -> (Vec<NoirParam>, Option<String>, Visibility) {
    let params_str = header
        .split('(')
        .nth(1)
        .and_then(|s| s.split(')').next())
        .unwrap_or("");

    let params = params_str
        .split(',')
        .filter_map(|p| {
            let p = p.trim();
            if p.is_empty() {
                return None;
            }
            // Noir marks visibility on the *type*, not the binding:
            // `fn claim(nullifier: pub Field, amount: u64)`.
            let mut parts = p.splitn(2, ':');
            let name = parts.next()?.trim().to_string();
            let ty = parts.next().unwrap_or("Field").trim();
            let (visibility, ty) = match ty.strip_prefix("pub ") {
                Some(stripped) => (Visibility::Public, stripped.trim()),
                None => (Visibility::Private, ty),
            };
            Some(NoirParam {
                name,
                ty: ty.to_string(),
                visibility,
            })
        })
        .collect();

    // Return type: `-> pub ReturnType` or `-> ReturnType`
    let (return_type, return_visibility) = if let Some(after_arrow) = header.split("->").nth(1) {
        let after_arrow = after_arrow.trim().trim_end_matches('{').trim();
        if after_arrow.starts_with("pub ") {
            (
                Some(after_arrow.trim_start_matches("pub ").trim().to_string()),
                Visibility::Public,
            )
        } else {
            (Some(after_arrow.to_string()), Visibility::Private)
        }
    } else {
        (None, Visibility::Private)
    };

    (params, return_type, return_visibility)
}

fn parse_statement(line: &str) -> NoirStatement {
    if line.starts_with("assert_eq(") {
        let inner = line.trim_start_matches("assert_eq(").trim_end_matches(';');
        let inner = inner.rsplit(')').nth(1).unwrap_or(inner);
        let mut parts = inner.splitn(2, ',');
        let lhs = parts.next().unwrap_or("").trim().to_string();
        let rhs = parts.next().unwrap_or("").trim().to_string();
        return NoirStatement::AssertEq { lhs, rhs };
    }

    if line.starts_with("assert(") {
        let inner = line
            .trim_start_matches("assert(")
            .split(',')
            .next()
            .unwrap_or("")
            .trim()
            .trim_end_matches(')')
            .to_string();
        return NoirStatement::Assert { expr: inner };
    }

    if line.contains("hash") && line.contains('(') {
        let callee = line.split('(').next().unwrap_or("").trim().to_string();
        let args_str = line
            .split('(')
            .nth(1)
            .and_then(|s| s.split(')').next())
            .unwrap_or("");
        let args = args_str.split(',').map(|a| a.trim().to_string()).collect();
        return NoirStatement::HashCall { callee, args };
    }

    if line.contains("storage.") {
        if line.contains(".get()") || line.contains(".read()") {
            let field = extract_storage_field(line);
            return NoirStatement::StorageRead { field };
        }
        if line.contains(".write(") || line.contains(".insert(") {
            let field = extract_storage_field(line);
            let value = line
                .split('(')
                .nth(1)
                .and_then(|s| s.split(')').next())
                .unwrap_or("")
                .to_string();
            return NoirStatement::StorageWrite { field, value };
        }
    }

    NoirStatement::Other {
        raw: line.to_string(),
    }
}

fn extract_storage_field(line: &str) -> String {
    line.split("storage.")
        .nth(1)
        .and_then(|s| s.split('.').next())
        .unwrap_or("")
        .to_string()
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_source_returns_error() {
        assert!(matches!(parse_noir(""), Err(NoirParseError::EmptySource)));
    }

    #[test]
    fn oversized_source_returns_error() {
        let big = "x".repeat(MAX_SOURCE_BYTES + 1);
        assert!(matches!(
            parse_noir(&big),
            Err(NoirParseError::SourceTooLarge { .. })
        ));
    }

    #[test]
    fn parses_simple_function_name() {
        let src = r#"
fn verify_membership(leaf: pub Field, root: pub Field) -> bool {
    assert(leaf != 0);
    true
}
"#;
        let module = parse_noir(src).unwrap();
        assert!(module.functions.contains_key("verify_membership"));
    }

    #[test]
    fn detects_public_params() {
        let src = r#"
fn claim(nullifier: pub Field, amount: u64) -> bool {
    assert(nullifier != 0);
    true
}
"#;
        let module = parse_noir(src).unwrap();
        let func = &module.functions["claim"];
        assert_eq!(func.params[0].visibility, Visibility::Public);
        assert_eq!(func.params[1].visibility, Visibility::Private);
    }

    #[test]
    fn parses_assert_statement() {
        let src = r#"
fn check(x: pub Field) {
    assert(x != 0);
}
"#;
        let module = parse_noir(src).unwrap();
        let func = &module.functions["check"];
        let has_assert = func
            .body
            .iter()
            .any(|s| matches!(s, NoirStatement::Assert { .. }));
        assert!(has_assert);
    }

    #[test]
    fn parses_hash_call() {
        let src = r#"
fn commit(secret: Field, blinding: Field) -> Field {
    std::hash::pedersen_hash([secret, blinding])
}
"#;
        let module = parse_noir(src).unwrap();
        let func = &module.functions["commit"];
        let has_hash = func
            .body
            .iter()
            .any(|s| matches!(s, NoirStatement::HashCall { .. }));
        assert!(has_hash);
    }

    #[test]
    fn parses_storage_read() {
        let src = r#"
fn get_root(storage: Storage) -> Field {
    storage.root.get()
}
"#;
        let module = parse_noir(src).unwrap();
        let func = &module.functions["get_root"];
        let has_read = func
            .body
            .iter()
            .any(|s| matches!(s, NoirStatement::StorageRead { .. }));
        assert!(has_read);
    }
}
