use std::path::Path;

use wot::model::{Language, NodeKind, SourceRange};
use wot::parsers::{dockerfile, hcl, notebook, xml};

#[test]
fn outlines_xml_elements_with_nested_ranges() {
    let source = r#"<root id="a">
  <child name="one">
    <leaf />
  </child>
</root>
"#;
    let outline = xml::parse(Path::new("layout.xml"), source, 4).unwrap();

    assert_eq!(outline.language, Language::Xml);
    assert_eq!(outline.nodes[0].label, "root id=\"a\"");
    assert_eq!(outline.nodes[0].kind, NodeKind::XmlElement);
    assert_eq!(outline.nodes[0].range, SourceRange::lines(1, 5));
    assert_eq!(outline.nodes[0].children[0].label, "child name=\"one\"");
    assert_eq!(outline.nodes[0].children[0].children[0].label, "leaf");
}

#[test]
fn reports_invalid_xml() {
    let error = xml::parse(Path::new("bad.xml"), "<root>", 3).unwrap_err();

    assert!(error.to_string().contains("bad.xml"));
}

#[test]
fn outlines_hcl_blocks_and_top_level_attributes() {
    let source = r#"variable "name" {
  default = "wot"
}

resource "local_file" "demo" {
  filename = "demo.txt"
}
"#;
    let outline = hcl::parse(Path::new("main.tf"), source, 3).unwrap();

    assert_eq!(outline.language, Language::Hcl);
    assert_eq!(outline.nodes[0].label, "variable \"name\"");
    assert_eq!(outline.nodes[0].kind, NodeKind::HclBlock);
    assert_eq!(outline.nodes[0].range, SourceRange::lines(1, 3));
    assert_eq!(outline.nodes[0].children[0].label, "default: \"wot\"");
    assert_eq!(outline.nodes[1].label, "resource \"local_file\" \"demo\"");
}

#[test]
fn outlines_dockerfile_stages_and_continuation_ranges() {
    let source = "FROM rust:1 AS builder\nRUN cargo build \\\n    --release\n\nFROM scratch\nCOPY --from=builder /app /app\n";
    let outline = dockerfile::parse(Path::new("Dockerfile"), source, 3).unwrap();

    assert_eq!(outline.language, Language::Dockerfile);
    assert_eq!(outline.nodes[0].label, "FROM rust:1 AS builder");
    assert_eq!(outline.nodes[0].kind, NodeKind::DockerStage);
    assert_eq!(outline.nodes[0].range, SourceRange::lines(1, 3));
    assert_eq!(
        outline.nodes[0].children[0].label,
        "RUN cargo build --release"
    );
    assert_eq!(outline.nodes[0].children[0].range, SourceRange::lines(2, 3));
    assert_eq!(outline.nodes[1].label, "FROM scratch");
}

#[test]
fn outlines_notebook_markdown_headings_and_python_symbols() {
    let source = r###"{
  "cells": [
    {
      "cell_type": "markdown",
      "source": ["# Intro\n", "## Details\n"]
    },
    {
      "cell_type": "code",
      "source": ["class Greeter:\n", "    def hello(self):\n", "        return 'hi'\n"]
    }
  ]
}"###;
    let outline = notebook::parse(Path::new("analysis.ipynb"), source, 4).unwrap();

    assert_eq!(outline.language, Language::Notebook);
    assert_eq!(outline.nodes[0].label, "markdown cell 1");
    assert_eq!(outline.nodes[0].kind, NodeKind::NotebookCell);
    assert_eq!(outline.nodes[0].children[0].label, "Intro");
    assert_eq!(outline.nodes[0].children[0].children[0].label, "Details");
    assert_eq!(outline.nodes[1].label, "code cell 2");
    assert_eq!(outline.nodes[1].children[0].label, "class Greeter");
    assert_eq!(outline.nodes[1].children[0].children[0].label, "def hello");
}

#[test]
fn reports_invalid_notebook_json() {
    let error = notebook::parse(Path::new("bad.ipynb"), "{", 3).unwrap_err();

    assert!(error.to_string().contains("bad.ipynb"));
}
