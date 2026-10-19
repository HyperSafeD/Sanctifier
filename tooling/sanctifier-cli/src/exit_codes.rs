/// Process exit code: analysis succeeded with no triggered findings.
pub const SUCCESS: i32 = 0;
/// Process exit code: findings were detected and the active profile triggered on them.
pub const FINDINGS_FOUND: i32 = 1;
/// Process exit code: unrecoverable error (invalid path, config parse failure, I/O error).
pub const ERROR: i32 = 2;
