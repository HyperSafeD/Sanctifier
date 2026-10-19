use std::fs;

#[test]
fn schemas_exist() {
    let files = vec![
        "schemas/security-review.schema.json",
        "schemas/sarif-rule-metadata.schema.json",
        "schemas/severity-taxonomy.schema.json",
    ];

    for file in files {
        assert!(fs::metadata(file).is_ok(), "Missing {}", file);
    }
}

#[test]
fn data_files_exist() {
    let files = vec![
        "data/security-review/owners.yaml",
        "data/security-review/checklist.yaml",
        "data/security-review/defaults.yaml",
        "data/sarif/rule-metadata.yaml",
        "data/sarif/severity-map.yaml",
    ];

    for file in files {
        assert!(fs::metadata(file).is_ok(), "Missing {}", file);
    }
}