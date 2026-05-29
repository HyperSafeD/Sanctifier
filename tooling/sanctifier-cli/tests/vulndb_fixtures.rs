use sanctifier_cli::vulndb::VulnDatabase;
use std::path::PathBuf;

fn vulndb_fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("vulndb")
        .join(name)
}

#[test]
fn minimal_vulndb_fixture_matches_todo_example() {
    let db = VulnDatabase::load(&vulndb_fixture_path("minimal-vulndb.json"))
        .expect("fixture minimal-vulndb.json should load");
    let source = std::fs::read_to_string(vulndb_fixture_path("todo_example.rs"))
        .expect("fixture todo_example.rs should load");

    let matches = db.scan(&source, "todo_example.rs");
    assert!(
        matches.iter().any(|m| m.vuln_id == "TEST-001"),
        "expected TEST-001 match, got {matches:?}"
    );
}

#[test]
fn custom_vulndb_fixture_matches_admin_example() {
    let db = VulnDatabase::load(&vulndb_fixture_path("custom-vulndb.json"))
        .expect("fixture custom-vulndb.json should load");
    let source = std::fs::read_to_string(vulndb_fixture_path("auth_admin_example.rs"))
        .expect("fixture auth_admin_example.rs should load");

    let matches = db.scan(&source, "auth_admin_example.rs");
    assert!(
        matches.iter().any(|m| m.vuln_id == "CUSTOM-AUTH-001"),
        "expected CUSTOM-AUTH-001 match, got {matches:?}"
    );
}
