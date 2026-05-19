use std::path::Path;

use wot::model::{Language, NodeKind, SourceRange};
use wot::parsers::markdown;

#[test]
fn outlines_nested_markdown_headings_with_section_ranges() {
    let source = "# Root\nintro\n## Child\nbody\n### Grandchild\nmore\n## Next\nend\n";
    let outline = markdown::parse(Path::new("doc.md"), source, 3).unwrap();

    assert_eq!(outline.language, Language::Markdown);
    assert_eq!(outline.nodes.len(), 1);
    assert_eq!(outline.nodes[0].label, "Root");
    assert_eq!(outline.nodes[0].kind, NodeKind::Heading);
    assert_eq!(outline.nodes[0].range, SourceRange::lines(1, 8));
    assert_eq!(outline.nodes[0].children[0].label, "Child");
    assert_eq!(outline.nodes[0].children[0].range, SourceRange::lines(3, 6));
    assert_eq!(outline.nodes[0].children[0].children[0].label, "Grandchild");
    assert_eq!(
        outline.nodes[0].children[0].children[0].range,
        SourceRange::lines(5, 6)
    );
    assert_eq!(outline.nodes[0].children[1].label, "Next");
    assert_eq!(outline.nodes[0].children[1].range, SourceRange::lines(7, 8));
}

#[test]
fn respects_max_depth_for_markdown_outline_depth() {
    let source = "# Root\n## Child\n### Grandchild\n";
    let outline = markdown::parse(Path::new("doc.md"), source, 2).unwrap();

    assert_eq!(outline.nodes.len(), 1);
    assert_eq!(outline.nodes[0].label, "Root");
    assert_eq!(outline.nodes[0].children.len(), 1);
    assert_eq!(outline.nodes[0].children[0].label, "Child");
    assert!(outline.nodes[0].children[0].children.is_empty());
}

#[test]
fn handles_skipped_heading_levels_as_outline_children() {
    let source = "# Root\n### Deep child\n## Sibling\n";
    let outline = markdown::parse(Path::new("doc.md"), source, 3).unwrap();

    assert_eq!(outline.nodes.len(), 1);
    assert_eq!(outline.nodes[0].children[0].label, "Deep child");
    assert_eq!(outline.nodes[0].children[1].label, "Sibling");
}

#[test]
fn returns_empty_outline_for_markdown_without_headings() {
    let source = "plain paragraph\n\nanother paragraph\n";
    let outline = markdown::parse(Path::new("doc.md"), source, 3).unwrap();

    assert!(outline.nodes.is_empty());
}
