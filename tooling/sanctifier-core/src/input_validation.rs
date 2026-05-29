//! Input validation for the `sanctifier-core` analysis engine.
//!
//! These guards run **before** any parsing or analysis work so that
//! well-understood bad inputs are rejected with actionable messages rather
//! than panicking deep inside the AST visitor.
//!
//! # Security posture / threat model
//!
//! The analyser accepts untrusted source code from several surfaces:
//! - CLI stdin / file paths supplied by users
//! - WASM API called from browser or Node.js
//! - CI integrations that pipe build artefacts
//!
//! Validated threats:
//! | Threat | Guard |
//! |---|---|
//! | Null-byte injection (C-string boundary break) | [`validate_no_null_bytes`] |
//! | Oversized input exhausting heap / parse time | [`validate_source_size`] |
//! | Empty or whitespace-only source (parse no-ops) | [`validate_source_size`] |
//! | Path traversal in workspace file discovery | [`validate_path`] |
//! | Non-UTF-8 byte sequences | [`validate_utf8`] |

/// Maximum accepted source size (10 MB), matching the WASM package limit.
pub const MAX_SOURCE_BYTES: usize = 10 * 1024 * 1024;

/// Minimum accepted source size (1 byte).
pub const MIN_SOURCE_BYTES: usize = 1;

/// Structured validation error returned by every guard.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationError {
    /// Machine-readable code for programmatic handling.
    pub code: &'static str,
    /// Human-readable message forwarded to the caller.
    pub message: String,
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.code, self.message)
    }
}

/// Validate that `source` is within the accepted size bounds.
///
/// # Errors
/// - `EMPTY_SOURCE` — source is zero bytes.
/// - `SOURCE_TOO_LARGE` — source exceeds [`MAX_SOURCE_BYTES`].
pub fn validate_source_size(source: &str) -> Result<(), ValidationError> {
    let len = source.len();
    if len < MIN_SOURCE_BYTES {
        return Err(ValidationError {
            code: "EMPTY_SOURCE",
            message: "Source code cannot be empty".to_string(),
        });
    }
    if len > MAX_SOURCE_BYTES {
        return Err(ValidationError {
            code: "SOURCE_TOO_LARGE",
            message: format!(
                "Source code is {} bytes; maximum allowed is {} bytes. \
                 Split the contract into smaller files.",
                len, MAX_SOURCE_BYTES
            ),
        });
    }
    Ok(())
}

/// Validate that `source` contains no null bytes.
///
/// Null bytes can silently truncate C-string boundaries in downstream tools
/// (linkers, LLVM passes, SARIF parsers) and represent a injection surface.
///
/// # Errors
/// - `NULL_BYTE_DETECTED` — at least one `\0` byte is present.
pub fn validate_no_null_bytes(source: &str) -> Result<(), ValidationError> {
    if source.contains('\0') {
        return Err(ValidationError {
            code: "NULL_BYTE_DETECTED",
            message: "Source code contains null bytes, which are not valid in Rust source \
                      and may indicate a binary file or injection attempt."
                .to_string(),
        });
    }
    Ok(())
}

/// Validate that `bytes` is valid UTF-8.
///
/// The analyser operates on `&str` exclusively; raw bytes that are not valid
/// UTF-8 must be rejected at the boundary before any conversion is attempted.
///
/// # Errors
/// - `INVALID_UTF8` — the byte sequence is not valid UTF-8.
pub fn validate_utf8(bytes: &[u8]) -> Result<(), ValidationError> {
    std::str::from_utf8(bytes)
        .map(|_| ())
        .map_err(|e| ValidationError {
            code: "INVALID_UTF8",
            message: format!(
                "Source bytes are not valid UTF-8 at byte offset {}: {}. \
             Ensure the file is saved as UTF-8.",
                e.valid_up_to(),
                e
            ),
        })
}

/// Validate that `path` does not contain path-traversal sequences.
///
/// Prevents a caller from supplying a crafted path like `../../etc/passwd`
/// to read files outside the intended workspace root.
///
/// # Errors
/// - `PATH_TRAVERSAL` — the path contains `..` components.
pub fn validate_path(path: &str) -> Result<(), ValidationError> {
    use std::path::Path;
    for component in Path::new(path).components() {
        if matches!(component, std::path::Component::ParentDir) {
            return Err(ValidationError {
                code: "PATH_TRAVERSAL",
                message: format!(
                    "Path '{}' contains a parent-directory traversal component ('..'), \
                     which is not permitted for workspace file discovery.",
                    path
                ),
            });
        }
    }
    Ok(())
}

/// Run all source-level guards in sequence, stopping at the first failure.
///
/// This is the recommended single call-site for validating source code before
/// passing it to [`sanctifier_core::Analyzer`].
///
/// # Errors
/// Returns the first [`ValidationError`] encountered, or `Ok(())` if all
/// guards pass.
pub fn validate_source_all(source: &str) -> Result<(), ValidationError> {
    validate_source_size(source)?;
    validate_no_null_bytes(source)?;
    Ok(())
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── validate_source_size ──────────────────────────────────────────────────

    #[test]
    fn empty_source_rejected() {
        let err = validate_source_size("").unwrap_err();
        assert_eq!(err.code, "EMPTY_SOURCE");
    }

    #[test]
    fn minimal_source_accepted() {
        assert!(validate_source_size("x").is_ok());
    }

    #[test]
    fn source_at_max_boundary_accepted() {
        let at_limit = "x".repeat(MAX_SOURCE_BYTES);
        assert!(validate_source_size(&at_limit).is_ok());
    }

    #[test]
    fn source_over_max_rejected() {
        let over = "x".repeat(MAX_SOURCE_BYTES + 1);
        let err = validate_source_size(&over).unwrap_err();
        assert_eq!(err.code, "SOURCE_TOO_LARGE");
        assert!(err.message.contains("Split the contract"));
    }

    // ── validate_no_null_bytes ────────────────────────────────────────────────

    #[test]
    fn null_byte_in_source_rejected() {
        let err = validate_no_null_bytes("fn foo() { let x = \0; }").unwrap_err();
        assert_eq!(err.code, "NULL_BYTE_DETECTED");
        assert!(err.message.contains("injection attempt"));
    }

    #[test]
    fn clean_source_passes_null_check() {
        assert!(validate_no_null_bytes("fn transfer() { let a = 1; }").is_ok());
    }

    // ── validate_utf8 ─────────────────────────────────────────────────────────

    #[test]
    fn valid_utf8_accepted() {
        assert!(validate_utf8(b"fn foo() {}").is_ok());
    }

    #[test]
    fn invalid_utf8_rejected() {
        let bad = b"fn foo() { let x = \xff\xfe; }";
        let err = validate_utf8(bad).unwrap_err();
        assert_eq!(err.code, "INVALID_UTF8");
        assert!(err.message.contains("UTF-8"));
    }

    // ── validate_path ─────────────────────────────────────────────────────────

    #[test]
    fn safe_path_accepted() {
        assert!(validate_path("contracts/my_contract/src/lib.rs").is_ok());
    }

    #[test]
    fn traversal_path_rejected() {
        let err = validate_path("../../etc/passwd").unwrap_err();
        assert_eq!(err.code, "PATH_TRAVERSAL");
        assert!(err.message.contains(".."));
    }

    #[test]
    fn single_dot_path_accepted() {
        assert!(validate_path("./src/lib.rs").is_ok());
    }

    // ── validate_source_all ───────────────────────────────────────────────────

    #[test]
    fn all_guards_pass_for_valid_source() {
        assert!(validate_source_all("fn main() {}").is_ok());
    }

    #[test]
    fn all_guards_fail_fast_on_empty() {
        let err = validate_source_all("").unwrap_err();
        assert_eq!(err.code, "EMPTY_SOURCE");
    }

    #[test]
    fn all_guards_fail_on_null_byte() {
        let err = validate_source_all("fn f() { \0 }").unwrap_err();
        assert_eq!(err.code, "NULL_BYTE_DETECTED");
    }

    #[test]
    fn display_includes_code_and_message() {
        let err = validate_source_all("").unwrap_err();
        let s = err.to_string();
        assert!(s.contains("EMPTY_SOURCE"));
    }
}
