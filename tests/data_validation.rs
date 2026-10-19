use std::fs;

#[test]
fn severity_map_contains_critical() {
    let content =
        fs::read_to_string("data/sarif/severity-map.yaml").unwrap();

    assert!(content.contains("critical"));
}