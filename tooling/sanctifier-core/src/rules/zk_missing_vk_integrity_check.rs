//! Z005 — Missing verifying-key integrity check before use.
//!
//! Where the verifying key lives in mutable contract storage — because the design
//! supports rotation — access control on the rotation function (Z010) is only half
//! the defence. It answers "who may write the key", not "is the key currently
//! stored the one we vetted". A storage-collision bug, an unrelated migration, or
//! a compromised admin can leave a hostile key in place, and every subsequent
//! `verify` silently accepts proofs forged under it.
//!
//! This rule flags verifier call sites that read the key from storage without a
//! preceding hash comparison or signature check.
//!
//! Verifying keys held in `const`/`static` items are out of scope: they cannot be
//! mutated at runtime, so a runtime integrity check is redundant by construction.
//! (Their own risk — undocumented provenance — is Z004's job.)
//!
//! See `docs/rules/Z005.md`.

use crate::rules::{Rule, RuleViolation, Severity};
use syn::{parse_str, File, Item, Local, Pat, Stmt};

/// Function names that perform on-chain proof verification.
const VERIFIER_FNS: &[&str] = &[
    "verify_proof",
    "groth16_verify",
    "verify_groth16",
    "snark_verify",
    "verify_snark",
    "plonk_verify",
    "verify_plonk",
    "verify_zk_proof",
];

/// Lowercased fragments identifying a verifying-key value.
const VK_FRAGMENTS: &[&str] = &[
    "verifying_key",
    "verification_key",
    "verifyingkey",
    "verificationkey",
    "vk",
];

/// Hash / signature primitives that can implement an integrity gate.
const INTEGRITY_PRIMITIVES: &[&str] = &[
    "sha256",
    "keccak256",
    "blake2",
    "hash",
    "ed25519_verify",
    "secp256k1_verify",
    "verify_sig",
];

/// Tokens that indicate the hash is being *compared*, not merely computed.
///
/// Loading a reference hash out of storage is not a check — only an actual
/// comparison or abort is. `.expect()` on a storage read is deliberately absent.
/// Both spaced and unspaced punctuation forms are listed because token-stream
/// rendering of `==` / `!=` is not guaranteed to be joint.
const COMPARISON_TOKENS: &[&str] = &[
    "assert_eq",
    "assert_ne",
    "assert !",
    "assert!",
    "==",
    "= =",
    "!=",
    "! =",
    "panic",
    "require",
];

/// Z005 — storage-loaded verifying key used without an integrity check.
pub struct ZkMissingVkIntegrityCheckRule;

impl ZkMissingVkIntegrityCheckRule {
    /// Create the rule.
    pub fn new() -> Self {
        Self
    }
}

impl Default for ZkMissingVkIntegrityCheckRule {
    fn default() -> Self {
        Self::new()
    }
}

/// Lowercased token soup for an arbitrary syn node.
fn tokens_lower<T: quote::ToTokens>(node: &T) -> String {
    quote::quote!(#node).to_string().to_lowercase()
}

/// Name bound by a `let` statement, if it is a simple identifier pattern.
fn binding_name(local: &Local) -> Option<String> {
    match &local.pat {
        Pat::Ident(id) => Some(id.ident.to_string()),
        Pat::Type(pt) => match &*pt.pat {
            Pat::Ident(id) => Some(id.ident.to_string()),
            _ => None,
        },
        _ => None,
    }
}

/// Does this text mention a verifying key?
fn mentions_vk(text: &str) -> bool {
    VK_FRAGMENTS.iter().any(|fragment| text.contains(fragment))
}

/// Does this text read from contract storage?
fn reads_storage(text: &str) -> bool {
    text.contains("storage")
        && (text.contains(". get") || text.contains(".get") || text.contains("get ("))
}

/// Does this statement constitute an integrity gate — a hash/signature check whose
/// result is actually compared or asserted?
fn is_integrity_gate(text: &str) -> bool {
    let has_primitive = INTEGRITY_PRIMITIVES.iter().any(|p| text.contains(p));
    if !has_primitive {
        return false;
    }
    let compares = COMPARISON_TOKENS
        .iter()
        .any(|token| text.contains(&token.to_lowercase()));
    compares && mentions_vk(text)
}

/// Analyse one function body. Returns `true` when a storage-loaded verifying key
/// reaches a verifier call with no integrity gate in between.
fn is_vulnerable(stmts: &[Stmt]) -> bool {
    let mut vk_from_storage: Option<String> = None;
    let mut checked = false;

    for stmt in stmts {
        let text = tokens_lower(stmt);

        // 1. Does this statement load the VK out of storage?
        if vk_from_storage.is_none() {
            if let Stmt::Local(local) = stmt {
                let init_text = local
                    .init
                    .as_ref()
                    .map(|i| tokens_lower(&*i.expr))
                    .unwrap_or_default();
                let name = binding_name(local).unwrap_or_default();
                let name_lower = name.to_lowercase();
                if reads_storage(&init_text)
                    && (mentions_vk(&init_text) || mentions_vk(&name_lower))
                {
                    vk_from_storage = Some(name_lower);
                    continue;
                }
            }
        }

        // 2. Is there an integrity gate before the verifier runs?
        if vk_from_storage.is_some() && !checked && is_integrity_gate(&text) {
            checked = true;
            continue;
        }

        // 3. Does the verifier run on that key?
        let calls_verifier = VERIFIER_FNS.iter().any(|f| text.contains(f));
        if calls_verifier {
            if let Some(vk_name) = &vk_from_storage {
                // Inline load: `groth16_verify(env.storage()...get(&VK), ..)`.
                let uses_vk = vk_name.is_empty() || text.contains(vk_name.as_str());
                if uses_vk && !checked {
                    return true;
                }
            } else if reads_storage(&text) && mentions_vk(&text) && !checked {
                return true;
            }
        }
    }

    false
}

impl Rule for ZkMissingVkIntegrityCheckRule {
    fn name(&self) -> &str {
        "missing_vk_integrity_check"
    }

    fn description(&self) -> &str {
        "Detects verifier calls that use a storage-loaded verifying key without checking its integrity against a known-good hash"
    }

    fn check(&self, source: &str) -> Vec<RuleViolation> {
        let file: File = match parse_str(source) {
            Ok(f) => f,
            Err(_) => return vec![],
        };

        let mut violations = Vec::new();

        for item in &file.items {
            let Item::Impl(impl_block) = item else {
                continue;
            };
            for impl_item in &impl_block.items {
                let syn::ImplItem::Fn(f) = impl_item else {
                    continue;
                };
                if !is_vulnerable(&f.block.stmts) {
                    continue;
                }

                let fn_name = f.sig.ident.to_string();
                violations.push(
                    RuleViolation::new(
                        self.name(),
                        Severity::High,
                        format!(
                            "Function '{}' loads the verifying key from mutable storage and passes it \
                             to the verifier without checking its integrity. If the stored key is ever \
                             swapped — by a compromised admin, a storage-key collision, or a migration \
                             bug — every proof verified here is forgeable.",
                            fn_name
                        ),
                        fn_name,
                    )
                    .with_suggestion(
                        "Commit a reference hash of the vetted verifying key at deployment, then hash \
                         the storage-loaded key and assert equality before calling the verifier, e.g. \
                         `assert_eq!(env.crypto().sha256(&vk_bytes), expected_vk_hash);`. Update the \
                         reference hash in the same transaction as any authorised rotation (Z010)."
                            .to_string(),
                    ),
                );
            }
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
    fn flags_storage_loaded_vk_without_check() {
        let rule = ZkMissingVkIntegrityCheckRule::new();
        let source = r#"
            impl Verifier {
                pub fn verify(env: Env, proof: Vec<u8>, inputs: Vec<u64>) -> bool {
                    let vk: BytesN<64> = env.storage().persistent().get(&DataKey::VerifyingKey).unwrap();
                    groth16_verify(vk.as_ref(), &proof, &inputs)
                }
            }
        "#;
        let v = rule.check(source);
        assert_eq!(v.len(), 1, "unchecked storage VK must fire: {v:?}");
        assert!(v[0].message.contains("verify"));
    }

    #[test]
    fn no_violation_when_hash_is_asserted() {
        let rule = ZkMissingVkIntegrityCheckRule::new();
        let source = r#"
            impl Verifier {
                pub fn verify(env: Env, proof: Vec<u8>, inputs: Vec<u64>) -> bool {
                    let vk: BytesN<64> = env.storage().persistent().get(&DataKey::VerifyingKey).unwrap();
                    let expected: BytesN<32> = env.storage().persistent().get(&DataKey::VkHash).unwrap();
                    assert_eq!(env.crypto().sha256(&Bytes::from_slice(&env, vk.as_ref())), expected, "vk integrity");
                    groth16_verify(vk.as_ref(), &proof, &inputs)
                }
            }
        "#;
        let v = rule.check(source);
        assert!(v.is_empty(), "checked storage VK must not fire: {v:?}");
    }

    #[test]
    fn no_violation_for_constant_vk() {
        let rule = ZkMissingVkIntegrityCheckRule::new();
        let source = r#"
            impl Verifier {
                pub fn verify(env: Env, proof: Vec<u8>, inputs: Vec<u64>) -> bool {
                    groth16_verify(VERIFYING_KEY, &proof, &inputs)
                }
            }
        "#;
        let v = rule.check(source);
        assert!(
            v.is_empty(),
            "an immutable constant key needs no runtime check: {v:?}"
        );
    }

    #[test]
    fn flags_inline_storage_read_in_verifier_call() {
        let rule = ZkMissingVkIntegrityCheckRule::new();
        let source = r#"
            impl Verifier {
                pub fn verify_inline(env: Env, proof: Vec<u8>) -> bool {
                    groth16_verify(
                        env.storage().persistent().get(&DataKey::VerifyingKey).unwrap(),
                        &proof,
                    )
                }
            }
        "#;
        let v = rule.check(source);
        assert_eq!(v.len(), 1, "inline storage read must fire: {v:?}");
    }

    #[test]
    fn no_violation_without_verifier_call() {
        let rule = ZkMissingVkIntegrityCheckRule::new();
        let source = r#"
            impl Verifier {
                pub fn get_vk(env: Env) -> BytesN<64> {
                    env.storage().persistent().get(&DataKey::VerifyingKey).unwrap()
                }
            }
        "#;
        assert!(rule.check(source).is_empty());
    }

    #[test]
    fn empty_and_unparseable_source_is_safe() {
        let rule = ZkMissingVkIntegrityCheckRule::new();
        assert!(rule.check("").is_empty());
        assert!(rule.check("not rust @@@").is_empty());
    }
}
