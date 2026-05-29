//! Core-type → [`Finding`](crate::types::Finding) conversion helpers.
//!
//! Each function takes a reference to a `sanctifier-core` issue type and
//! returns a normalised [`Finding`] suitable for JS consumers.  Keeping these
//! here rather than inlining them into the analysis module makes it easy to
//! audit the mapping between internal codes and output messages in one place.

use sanctifier_core::{
    finding_codes, ArithmeticIssue, AuthGapIssue, EventIssue, PanicIssue, SizeWarning,
    StorageCollisionIssue, UnhandledResultIssue, UnsafePattern,
};

use crate::types::Finding;

pub fn auth_gap(issue: &AuthGapIssue) -> Finding {
    Finding {
        code: finding_codes::AUTH_GAP,
        category: "authentication",
        message: format!("Missing authentication guard in `{}`", issue.function_name),
        location: Some(issue.function_name.clone()),
    }
}

pub fn panic_issue(p: &PanicIssue) -> Finding {
    Finding {
        code: finding_codes::PANIC_USAGE,
        category: "panic_handling",
        message: format!("`{}` usage in `{}`", p.issue_type, p.function_name),
        location: Some(p.location.clone()),
    }
}

pub fn arithmetic(a: &ArithmeticIssue) -> Finding {
    Finding {
        code: finding_codes::ARITHMETIC_OVERFLOW,
        category: "arithmetic",
        message: format!(
            "Unchecked `{}` in `{}` — {}",
            a.operation, a.function_name, a.suggestion
        ),
        location: Some(a.location.clone()),
    }
}

pub fn size_warning(w: &SizeWarning) -> Finding {
    Finding {
        code: finding_codes::LEDGER_SIZE_RISK,
        category: "storage_limits",
        message: format!(
            "`{}` estimated size {}B approaches/exceeds ledger limit {}B",
            w.struct_name, w.estimated_size, w.limit
        ),
        location: None,
    }
}

pub fn unsafe_pattern(p: &UnsafePattern) -> Finding {
    Finding {
        code: finding_codes::UNSAFE_PATTERN,
        category: "unsafe_patterns",
        message: format!("{:?} at line {}: {}", p.pattern_type, p.line, p.snippet),
        location: Some(format!("line:{}", p.line)),
    }
}

pub fn storage_collision(c: &StorageCollisionIssue) -> Finding {
    Finding {
        code: finding_codes::STORAGE_COLLISION,
        category: "storage_keys",
        message: c.message.clone(),
        location: Some(c.location.clone()),
    }
}

pub fn event_issue(e: &EventIssue) -> Finding {
    Finding {
        code: finding_codes::EVENT_INCONSISTENCY,
        category: "events",
        message: e.message.clone(),
        location: Some(e.location.clone()),
    }
}

pub fn unhandled_result(r: &UnhandledResultIssue) -> Finding {
    Finding {
        code: finding_codes::UNHANDLED_RESULT,
        category: "logic",
        message: r.message.clone(),
        location: Some(r.location.clone()),
    }
}
