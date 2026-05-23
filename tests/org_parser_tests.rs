use std::path::Path;

use wot::model::{Language, NodeKind, SourceRange};
use wot::parsers::org;

#[test]
fn outlines_nested_org_headings_with_section_ranges() {
    let source =
        "* Root\nintro\n** TODO Child :tag:\nbody\n*** [#A] Grandchild\nmore\n** Next\nend\n";
    let outline = org::parse(Path::new("notes.org"), source, 3).unwrap();

    assert_eq!(outline.language, Language::Org);
    assert_eq!(outline.nodes.len(), 1);
    assert_eq!(outline.nodes[0].label, "Root");
    assert_eq!(outline.nodes[0].kind, NodeKind::Heading);
    assert_eq!(outline.nodes[0].range, SourceRange::lines(1, 8));
    assert_eq!(outline.nodes[0].children[0].label, "TODO Child :tag:");
    assert_eq!(outline.nodes[0].children[0].range, SourceRange::lines(3, 6));
    assert_eq!(
        outline.nodes[0].children[0].children[0].label,
        "[#A] Grandchild"
    );
    assert_eq!(
        outline.nodes[0].children[0].children[0].range,
        SourceRange::lines(5, 6)
    );
    assert_eq!(outline.nodes[0].children[1].label, "Next");
    assert_eq!(outline.nodes[0].children[1].range, SourceRange::lines(7, 8));
}

#[test]
fn respects_max_depth_for_org_outline_depth() {
    let source = "* Root\n** Child\n*** Grandchild\n";
    let outline = org::parse(Path::new("notes.org"), source, 2).unwrap();

    assert_eq!(outline.nodes.len(), 1);
    assert_eq!(outline.nodes[0].label, "Root");
    assert_eq!(outline.nodes[0].children.len(), 1);
    assert_eq!(outline.nodes[0].children[0].label, "Child");
    assert!(outline.nodes[0].children[0].children.is_empty());
}

#[test]
fn handles_skipped_org_heading_levels_as_outline_children() {
    let source = "* Root\n*** Deep child\n** Sibling\n";
    let outline = org::parse(Path::new("notes.org"), source, 3).unwrap();

    assert_eq!(outline.nodes.len(), 1);
    assert_eq!(outline.nodes[0].children[0].label, "Deep child");
    assert_eq!(outline.nodes[0].children[1].label, "Sibling");
}

#[test]
fn ignores_non_heading_star_lines_and_block_contents() {
    let source = "* Root\n*not a heading\n#+begin_src org\n* Ignored\n#+end_src\n** Child\n";
    let outline = org::parse(Path::new("notes.org"), source, 3).unwrap();

    assert_eq!(outline.nodes.len(), 1);
    assert_eq!(outline.nodes[0].label, "Root");
    assert_eq!(outline.nodes[0].range, SourceRange::lines(1, 6));
    assert_eq!(outline.nodes[0].children.len(), 1);
    assert_eq!(outline.nodes[0].children[0].label, "Child");
    assert_eq!(outline.nodes[0].children[0].range, SourceRange::lines(6, 6));
}

#[test]
fn returns_empty_outline_for_org_without_headings() {
    let source = "plain paragraph\n\nanother paragraph\n";
    let outline = org::parse(Path::new("notes.org"), source, 3).unwrap();

    assert!(outline.nodes.is_empty());
}
