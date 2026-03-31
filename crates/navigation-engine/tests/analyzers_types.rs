use navigation_engine::analyzers::types::normalize_public_symbol_kind;

#[test]
fn normalizes_internal_kind_aliases_to_public_kinds() {
    assert_eq!(normalize_public_symbol_kind("class_declaration"), "class");
    assert_eq!(normalize_public_symbol_kind("method_declaration"), "method");
    assert_eq!(normalize_public_symbol_kind("type_alias"), "type");
    assert_eq!(normalize_public_symbol_kind("record"), "type");
    assert_eq!(
        normalize_public_symbol_kind("annotation_type"),
        "annotation"
    );
}

#[test]
fn unknown_kind_degrades_to_any() {
    assert_eq!(normalize_public_symbol_kind("totally_unknown"), "any");
}
