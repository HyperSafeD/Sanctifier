// sanctifier-ignore: intentional test fixture — TODO comments here are
// required to trigger the TEST-001 rule in minimal-vulndb.json.
// Any repo-wide "no leftover TODO" policy (pre-commit hook, CI lint, etc.)
// must allowlist this file rather than removing the comments.
//
// Example Rust source file with TODO comments (triggers TEST-001 from minimal-vulndb.json)

pub fn incomplete_function() {
    // TODO: Implement proper error handling
    let result = risky_operation();

    // TODO: Add logging here
    process_result(result);
}

fn risky_operation() -> i32 {
    // TODO: Validate input parameters
    42
}

fn process_result(value: i32) {
    // TODO: Handle edge cases
    println!("Result: {}", value);
}
