use std::path::Path;

use wot::error::Error;
use wot::model::{Language, NodeKind, SourceRange};
use wot::parsers::json;

#[test]
fn outlines_json_objects_arrays_and_scalar_previews() {
    let source = r#"{
  "name": "Ada",
  "items": [
    {"id": 1},
    {"id": 2}
  ]
}"#;
    let outline = json::parse(Path::new("data.json"), source, 3).unwrap();

    assert_eq!(outline.language, Language::Json);
    assert_eq!(outline.nodes.len(), 2);
    assert_eq!(outline.nodes[0].label, "name: \"Ada\"");
    assert_eq!(outline.nodes[0].kind, NodeKind::JsonProperty);
    assert_eq!(outline.nodes[0].range, SourceRange::lines(2, 2));
    assert_eq!(outline.nodes[1].label, "items: array[2]");
    assert_eq!(outline.nodes[1].range, SourceRange::lines(3, 6));
    assert_eq!(outline.nodes[1].children[0].label, "[0]: object");
    assert_eq!(outline.nodes[1].children[0].range, SourceRange::lines(4, 4));
    assert_eq!(outline.nodes[1].children[0].children[0].label, "id: 1");
}

#[test]
fn uses_precise_ranges_for_same_line_json_members() {
    let source = r#"{"a":1,"b":2}"#;
    let outline = json::parse(Path::new("data.json"), source, 2).unwrap();

    assert_eq!(outline.nodes.len(), 2);
    assert_eq!(outline.nodes[0].range, SourceRange::spans(1, 2, 1, 7));
    assert_eq!(outline.nodes[1].range, SourceRange::spans(1, 8, 1, 13));
}

#[test]
fn respects_max_depth_for_json_outline_depth() {
    let source = r#"{"outer":{"inner":{"leaf":1}}}"#;
    let outline = json::parse(Path::new("data.json"), source, 2).unwrap();

    assert_eq!(outline.nodes.len(), 1);
    assert_eq!(outline.nodes[0].label, "outer: object");
    assert_eq!(outline.nodes[0].children[0].label, "inner: object");
    assert!(outline.nodes[0].children[0].children.is_empty());
}

#[test]
fn reports_invalid_json() {
    let error = json::parse(Path::new("bad.json"), r#"{"a":}"#, 3).unwrap_err();

    assert!(matches!(error, Error::Parse { .. }));
    assert!(error.to_string().contains("bad.json"));
}
