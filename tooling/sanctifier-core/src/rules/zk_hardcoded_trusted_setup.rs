//! Z004 — Unverified or hardcoded trusted-setup parameters ("toxic waste").
//!
//! Groth16 and comparable schemes require a trusted setup whose secret randomness
//! — the "toxic waste" — must be provably destroyed. A verifying key baked into
//! contract source with no pointer to a public ceremony transcript cannot be
//! audited: nobody outside the deploying team can tell whether the trapdoor still
//! exists. This rule flags *undocumented* setup material.
//!
//! It deliberately checks provenance only. Whether a referenced transcript is
//! cryptographically sound is out of scope — that requires replaying the ceremony,
//! not reading the contract.
//!
//! Detection runs over raw source text rather than the `syn` AST because ordinary
//! `//` comments — the very thing that carries the provenance — are discarded
//! during parsing.
//!
//! See `docs/rules/Z004.md`.

use crate::rules::{Rule, RuleViolation, Severity};

/// Uppercased fragments of constant names that denote trusted-setup material.
const SETUP_NAME_FRAGMENTS: &[&str] = &[
    "VERIFYING_KEY",
    "VERIFICATION_KEY",
    "VK_",
    "_VK",
    "PROVING_KEY",
    "TRUSTED_SETUP",
    "SETUP_PARAMS",
    "ALPHA_G1",
    "BETA_G2",
    "GAMMA_G2",
    "DELTA_G2",
    "GAMMA_ABC",
    "SRS_",
    "_SRS",
    "CRS_",
    "_CRS",
    "TAU_",
    "POWERS_OF_TAU",
];

/// Lowercased markers that establish ceremony provenance in a comment block.
const PROVENANCE_MARKERS: &[&str] = &[
    "ceremony",
    "transcript",
    "attestation",
    "powers of tau",
    "powersoftau",
    "ptau",
    "phase2",
    "phase 2",
    "perpetual powers",
    "trusted setup:",
    "setup ceremony",
    "mpc",
    "contribution hash",
];

/// Z004 — hardcoded verifying-key material without ceremony provenance.
pub struct ZkHardcodedTrustedSetupRule;

impl ZkHardcodedTrustedSetupRule {
    /// Create the rule.
    pub fn new() -> Self {
        Self
    }
}

impl Default for ZkHardcodedTrustedSetupRule {
    fn default() -> Self {
        Self::new()
    }
}

/// Does this line declare a `const`/`static` item?
fn declaration_name(line: &str) -> Option<&str> {
    let trimmed = line.trim_start();
    let rest = trimmed
        .strip_prefix("pub const ")
        .or_else(|| trimmed.strip_prefix("pub static "))
        .or_else(|| trimmed.strip_prefix("const "))
        .or_else(|| trimmed.strip_prefix("static "))
        .or_else(|| {
            trimmed
                .strip_prefix("pub(crate) const ")
                .or_else(|| trimmed.strip_prefix("pub(crate) static "))
        })?;
    let name = rest.split(':').next()?.trim();
    let name = name.split('=').next()?.trim();
    if name.is_empty() || name.contains(char::is_whitespace) {
        None
    } else {
        Some(name)
    }
}

/// Does the constant's name look like trusted-setup material?
fn is_setup_name(name: &str) -> bool {
    let upper = name.to_uppercase();
    // Bare `VK` is a common shorthand and would not match the `VK_`/`_VK`
    // fragments, so allow it explicitly.
    if upper == "VK" || upper == "SRS" || upper == "CRS" {
        return true;
    }
    SETUP_NAME_FRAGMENTS
        .iter()
        .any(|fragment| upper.contains(fragment))
}

/// Does the initializer look like raw key material (a byte/hex literal)?
///
/// `value` is the text from the `=` up to and including the terminating `;`,
/// which may span several lines.
fn looks_like_key_material(value: &str) -> bool {
    let has_hex = value.contains("0x") || value.contains("0X");
    let has_array = value.contains('[');
    let long_hex_string = value
        .split('"')
        .any(|chunk| chunk.len() >= 32 && chunk.chars().all(|c| c.is_ascii_hexdigit()));

    (has_array && (has_hex || value.matches(',').count() >= 3)) || long_hex_string
}

/// Walk backwards from `idx` collecting the contiguous comment/attribute block
/// that immediately precedes the declaration.
fn preceding_comment_block(lines: &[&str], idx: usize) -> String {
    let mut block = Vec::new();
    let mut cursor = idx;
    while cursor > 0 {
        cursor -= 1;
        let trimmed = lines[cursor].trim();
        if trimmed.starts_with("//")
            || trimmed.starts_with("#[")
            || trimmed.starts_with("*")
            || trimmed.starts_with("/*")
            || trimmed.ends_with("*/")
        {
            block.push(trimmed);
        } else if trimmed.is_empty() && !block.is_empty() {
            // A blank line inside a doc block is still part of it; a blank line
            // before any comment means there is no adjacent documentation.
            block.push(trimmed);
        } else {
            break;
        }
    }
    block.reverse();
    block.join("\n")
}

/// Phrases that turn a provenance marker into a statement that provenance is
/// *absent*. "no ceremony transcript" must not read as documented provenance.
const NEGATIONS: &[&str] = &[
    "no ",
    "not ",
    "never ",
    "without ",
    "missing ",
    "lacks ",
    "lacking ",
    "undocumented",
    "unverified",
    "absent",
];

/// Does a comment block carry ceremony provenance?
///
/// A marker only counts when it is not negated earlier on the same line, so a
/// comment explaining that the key has *no* transcript does not suppress the
/// finding.
fn has_provenance(block: &str) -> bool {
    block.lines().any(|line| {
        let lower = line.to_lowercase();
        PROVENANCE_MARKERS.iter().any(|marker| {
            let Some(pos) = lower.find(marker) else {
                return false;
            };
            let preceding = &lower[..pos];
            !NEGATIONS.iter().any(|neg| preceding.contains(neg))
        })
    })
}

impl Rule for ZkHardcodedTrustedSetupRule {
    fn name(&self) -> &str {
        "hardcoded_trusted_setup"
    }

    fn description(&self) -> &str {
        "Detects hardcoded verifying-key / trusted-setup material with no reference to an auditable ceremony transcript"
    }

    fn check(&self, source: &str) -> Vec<RuleViolation> {
        let lines: Vec<&str> = source.lines().collect();
        let mut violations = Vec::new();

        for (idx, line) in lines.iter().enumerate() {
            let Some(name) = declaration_name(line) else {
                continue;
            };
            if !is_setup_name(name) {
                continue;
            }

            // Gather the initializer, which frequently wraps across many lines
            // for a 256-byte key. Start after the `=` so that the `;` inside an
            // array type such as `[u8; 32]` does not terminate the scan early.
            let Some((_, first)) = line.split_once('=') else {
                continue;
            };
            let mut value = String::from(first);
            value.push('\n');
            if !first.contains(';') {
                for candidate in lines.iter().skip(idx + 1) {
                    value.push_str(candidate);
                    value.push('\n');
                    if candidate.contains(';') {
                        break;
                    }
                    // Guard against a runaway scan on a malformed file.
                    if value.len() > 20_000 {
                        break;
                    }
                }
            }

            if !looks_like_key_material(&value) {
                continue;
            }

            let block = preceding_comment_block(&lines, idx);
            if has_provenance(&block) {
                continue;
            }

            violations.push(
                RuleViolation::new(
                    self.name(),
                    Severity::Critical,
                    format!(
                        "Trusted-setup constant '{}' is hardcoded at line {} with no reference to a \
                         public ceremony transcript. Without documented provenance there is no way \
                         to audit whether the setup's toxic waste was destroyed, so the key cannot \
                         be trusted for mainnet value.",
                        name,
                        idx + 1
                    ),
                    format!("{}:{}", name, idx + 1),
                )
                .with_suggestion(
                    "Add a comment directly above the constant recording the ceremony provenance, \
                     e.g. `// ceremony: perpetual powers-of-tau phase2, contribution #47, \
                     transcript sha256 <hash>, https://ceremony.example/transcript.json`. \
                     Alternatively load the verifying key from governance-controlled storage so it \
                     can be rotated (Z010) and integrity-checked at each use (Z005)."
                        .to_string(),
                ),
            );
        }

        violations
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flags_undocumented_hardcoded_verifying_key() {
        let rule = ZkHardcodedTrustedSetupRule::new();
        let source = r#"
const VK_ALPHA_G1: [u8; 8] = [0xde, 0xad, 0xbe, 0xef, 0x00, 0x11, 0x22, 0x33];

impl Verifier {
    pub fn verify(env: Env, proof: Vec<u8>) -> bool {
        groth16_verify(&VK_ALPHA_G1, &proof)
    }
}
"#;
        let v = rule.check(source);
        assert_eq!(v.len(), 1, "undocumented VK must fire once: {v:?}");
        assert!(v[0].message.contains("VK_ALPHA_G1"));
    }

    #[test]
    fn no_violation_when_ceremony_is_documented() {
        let rule = ZkHardcodedTrustedSetupRule::new();
        let source = r#"
// ceremony: perpetual powers-of-tau phase2, contribution #47
// transcript sha256: 3f7a...c19e
// https://ceremony.example.org/transcripts/47.json
const VK_ALPHA_G1: [u8; 8] = [0xde, 0xad, 0xbe, 0xef, 0x00, 0x11, 0x22, 0x33];
"#;
        assert!(
            rule.check(source).is_empty(),
            "documented ceremony must not fire"
        );
    }

    #[test]
    fn doc_comment_provenance_is_accepted() {
        let rule = ZkHardcodedTrustedSetupRule::new();
        let source = r#"
/// Verifying key from the Hermez powersOfTau28 ceremony transcript.
pub const VERIFYING_KEY: &[u8] = &[0x01, 0x02, 0x03, 0x04];
"#;
        assert!(rule.check(source).is_empty());
    }

    #[test]
    fn ignores_unrelated_constants() {
        let rule = ZkHardcodedTrustedSetupRule::new();
        let source = r#"
const MAX_ENTRIES: u32 = 100;
const DEFAULT_SALT: [u8; 4] = [0x00, 0x01, 0x02, 0x03];
"#;
        assert!(rule.check(source).is_empty());
    }

    #[test]
    fn ignores_setup_names_without_key_material() {
        let rule = ZkHardcodedTrustedSetupRule::new();
        // A storage key symbol, not the key itself.
        let source = r#"
const VK_STORAGE_KEY: &str = "VK";
"#;
        assert!(rule.check(source).is_empty());
    }

    #[test]
    fn handles_multi_line_key_literals() {
        let rule = ZkHardcodedTrustedSetupRule::new();
        let source = r#"
const TRUSTED_SETUP_PARAMS: [u8; 6] = [
    0xaa, 0xbb,
    0xcc, 0xdd,
    0xee, 0xff,
];
"#;
        let v = rule.check(source);
        assert_eq!(v.len(), 1, "multi-line literal must still fire: {v:?}");
    }

    #[test]
    fn negated_provenance_does_not_suppress_the_finding() {
        let rule = ZkHardcodedTrustedSetupRule::new();
        // A comment stating that provenance is missing is not provenance.
        let source = r#"
// VULNERABLE: key material with no ceremony reference anywhere near it.
const VK_ALPHA_G1: [u8; 4] = [0xde, 0xad, 0xbe, 0xef];
"#;
        let v = rule.check(source);
        assert_eq!(v.len(), 1, "negated marker must still fire: {v:?}");
    }

    #[test]
    fn empty_source_is_safe() {
        assert!(ZkHardcodedTrustedSetupRule::new().check("").is_empty());
    }
}
