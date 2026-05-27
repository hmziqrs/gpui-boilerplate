#[test]
fn qa_matrix_contains_core_cases() {
    let content = std::fs::read_to_string("docs/qa-matrix.md").expect("read qa matrix");
    let normalized = content.to_lowercase();
    assert!(normalized.contains("second-instance forwarding"));
    assert!(normalized.contains("open logs folder"));
    assert!(normalized.contains("secure storage unavailable path"));
}

#[test]
fn accessibility_checklist_exists() {
    let content = std::fs::read_to_string("docs/accessibility-checklist.md")
        .expect("read accessibility checklist");
    assert!(content.contains("Keyboard And Focus"));
    assert!(content.contains("Done Criteria"));
}
