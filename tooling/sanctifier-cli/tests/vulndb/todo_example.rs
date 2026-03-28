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
