use super::{
    response::{parse_json, parse_xml_preview, truncate},
    *,
};

#[test]
fn parse_helpers_split_json_xml_and_text() {
    assert!(parse_json("{\"ok\":true}").is_some());
    assert!(parse_json("<root />").is_none());
    assert!(parse_xml_preview("<root />", HttpBodyKind::Xml).is_some());
    assert!(parse_xml_preview("<html />", HttpBodyKind::Text).is_none());
}

#[test]
fn truncate_preserves_utf8_boundaries() {
    let value = "hello ڄاڻ world";
    let truncated = truncate(value, 9);
    assert!(truncated.ends_with('…'));
    assert!(std::str::from_utf8(truncated.as_bytes()).is_ok());
}
