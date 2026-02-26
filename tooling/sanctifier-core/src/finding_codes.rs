use serde::Serialize;

pub const AUTH_GAP: &str = "S001";
pub const PANIC_USAGE: &str = "S002";
pub const ARITHMETIC_OVERFLOW: &str = "S003";
pub const LEDGER_SIZE_RISK: &str = "S004";
pub const STORAGE_COLLISION: &str = "S005";
pub const UNSAFE_PATTERN: &str = "S006";
pub const CUSTOM_RULE_MATCH: &str = "S007";

#[derive(Debug, Clone, Serialize)]
pub struct FindingCode {
    pub code: &'static str,
    pub category: &'static str,
    pub description: &'static str,
}

pub fn all_finding_codes() -> Vec<FindingCode> {
    vec![
        FindingCode {
            code: AUTH_GAP,
            category: "authentication",
            description: "Missing authentication guard in a state-mutating function",
        },
        FindingCode {
            code: PANIC_USAGE,
            category: "panic_handling",
            description: "panic!/unwrap/expect usage that may cause runtime aborts",
        },
        FindingCode {
            code: ARITHMETIC_OVERFLOW,
            category: "arithmetic",
            description: "Unchecked arithmetic operation with overflow/underflow risk",
        },
        FindingCode {
            code: LEDGER_SIZE_RISK,
            category: "storage_limits",
            description: "Ledger entry size is exceeding or approaching configured threshold",
        },
        FindingCode {
            code: STORAGE_COLLISION,
            category: "storage_keys",
            description: "Potential storage key collision across contract data paths",
        },
        FindingCode {
            code: UNSAFE_PATTERN,
            category: "unsafe_patterns",
            description: "Potentially unsafe language/runtime pattern was detected",
        },
        FindingCode {
            code: CUSTOM_RULE_MATCH,
            category: "custom_rule",
            description: "User-defined rule matched contract source",
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn finding_codes_are_unique() {
        let codes = all_finding_codes();
        let unique: HashSet<&str> = codes.iter().map(|c| c.code).collect();
        assert_eq!(codes.len(), unique.len());
    }

    #[test]
    fn includes_expected_codes() {
        let codes = all_finding_codes();
        assert!(codes.iter().any(|c| c.code == AUTH_GAP));
        assert!(codes.iter().any(|c| c.code == PANIC_USAGE));
        assert!(codes.iter().any(|c| c.code == ARITHMETIC_OVERFLOW));
        assert!(codes.iter().any(|c| c.code == LEDGER_SIZE_RISK));
        assert!(codes.iter().any(|c| c.code == STORAGE_COLLISION));
        assert!(codes.iter().any(|c| c.code == UNSAFE_PATTERN));
        assert!(codes.iter().any(|c| c.code == CUSTOM_RULE_MATCH));
    }
}
