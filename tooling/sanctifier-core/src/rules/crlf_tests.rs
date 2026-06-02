//! CRLF / line-ending tolerance tests for all parser-backed rules.
//!
//! Windows editors and git checkouts may produce CRLF (`\r\n`) line endings.
//! Every rule must produce identical results whether the input uses LF or CRLF.

#[cfg(test)]
mod crlf_tests {
    use crate::rules::{
        instance_storage_misuse::InstanceStorageMisuseRule,
        panic_detection::PanicDetectionRule,
        reentrancy::ReentrancyRule,
        unhandled_result::UnhandledResultRule,
        Rule,
    };

    /// Convert LF source to CRLF by replacing every `\n` with `\r\n`.
    fn to_crlf(s: &str) -> String {
        s.replace('\n', "\r\n")
    }

    macro_rules! assert_crlf_parity {
        ($rule:expr, $source:expr, $label:expr) => {{
            let lf_violations   = $rule.check($source);
            let crlf_source     = to_crlf($source);
            let crlf_violations = $rule.check(&crlf_source);
            assert_eq!(
                lf_violations.len(),
                crlf_violations.len(),
                "{}: violation count differs between LF and CRLF input",
                $label,
            );
        }};
    }

    // ── PanicDetectionRule ────────────────────────────────────────────────────

    const PANIC_SOURCE: &str = r#"
impl Contract {
    pub fn transfer(e: Env) {
        let x: Option<u32> = None;
        let _ = x.unwrap();
    }
}
"#;

    #[test]
    fn panic_detection_lf_crlf_parity() {
        assert_crlf_parity!(PanicDetectionRule::new(), PANIC_SOURCE, "PanicDetectionRule");
    }

    #[test]
    fn panic_detection_crlf_finds_violations() {
        let rule = PanicDetectionRule::new();
        let crlf = to_crlf(PANIC_SOURCE);
        let v = rule.check(&crlf);
        assert!(!v.is_empty(), "PanicDetectionRule must flag unwrap() in CRLF input");
    }

    // ── UnhandledResultRule ───────────────────────────────────────────────────

    const UNHANDLED_SOURCE: &str = r#"
impl Contract {
    pub fn do_work(e: Env) {
        some_fallible_call();
    }
}
fn some_fallible_call() -> Result<(), ()> { Ok(()) }
"#;

    #[test]
    fn unhandled_result_lf_crlf_parity() {
        assert_crlf_parity!(UnhandledResultRule::new(), UNHANDLED_SOURCE, "UnhandledResultRule");
    }

    // ── InstanceStorageMisuseRule ─────────────────────────────────────────────

    const STORAGE_SOURCE: &str = r#"
impl Contract {
    pub fn store(e: Env) {
        e.storage().instance().set(&1u32, &"value");
    }
}
"#;

    #[test]
    fn instance_storage_misuse_lf_crlf_parity() {
        assert_crlf_parity!(
            InstanceStorageMisuseRule::new(),
            STORAGE_SOURCE,
            "InstanceStorageMisuseRule"
        );
    }

    // ── ReentrancyRule ────────────────────────────────────────────────────────

    const REENTRANCY_SOURCE: &str = r#"
impl Contract {
    pub fn call_external(e: Env, addr: Address) {
        let client = SomeClient::new(&e, &addr);
        client.some_method();
        e.storage().instance().set(&1u32, &true);
    }
}
"#;

    #[test]
    fn reentrancy_lf_crlf_parity() {
        assert_crlf_parity!(ReentrancyRule::new(), REENTRANCY_SOURCE, "ReentrancyRule");
    }

    // ── Clean source: no violations on either ending ──────────────────────────

    const CLEAN_SOURCE: &str = r#"
impl Contract {
    pub fn safe(e: Env) -> Result<(), Error> {
        Ok(())
    }
}
"#;

    #[test]
    fn clean_source_no_violations_lf() {
        let v = PanicDetectionRule::new().check(CLEAN_SOURCE);
        assert!(v.is_empty(), "clean LF source must have no violations");
    }

    #[test]
    fn clean_source_no_violations_crlf() {
        let crlf = to_crlf(CLEAN_SOURCE);
        let v = PanicDetectionRule::new().check(&crlf);
        assert!(v.is_empty(), "clean CRLF source must have no violations");
    }
}
