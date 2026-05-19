use wot::model::{Language, NodeKind, Outline, OutlineNode, SourceRange};
use wot::renderer::render_markdown;

#[test]
fn renders_file_header_and_nested_nodes_with_ranges() {
    let outline = Outline {
        path: "sample.py".into(),
        language: Language::Python,
        nodes: vec![OutlineNode {
            label: "class Greeter".into(),
            kind: NodeKind::Class,
            range: SourceRange::lines(1, 8),
            children: vec![OutlineNode {
                label: "def hello".into(),
                kind: NodeKind::Function,
                range: SourceRange::lines(2, 4),
                children: vec![],
            }],
        }],
    };

    assert_eq!(
        render_markdown(&outline),
        "# sample.py\n- class Greeter [L1-L8]\n  - def hello [L2-L4]\n"
    );
}

#[test]
fn renders_precise_single_line_ranges_with_columns() {
    let outline = Outline {
        path: "data.json".into(),
        language: Language::Json,
        nodes: vec![OutlineNode {
            label: "a: 1".into(),
            kind: NodeKind::JsonProperty,
            range: SourceRange::spans(1, 2, 1, 7),
            children: vec![],
        }],
    };

    assert_eq!(
        render_markdown(&outline),
        "# data.json\n- a: 1 [L1:C2-L1:C7]\n"
    );
}
