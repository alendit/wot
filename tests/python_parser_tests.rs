use std::path::Path;

use wot::error::Error;
use wot::model::{Language, NodeKind, SourceRange};
use wot::parsers::python;

#[test]
fn outlines_python_classes_methods_functions_and_nested_defs() {
    let source = "\
class Greeter:
    def hello(self):
        def format_name():
            return 'Ada'
        return format_name()

async def fetch():
    return 1
";
    let outline = python::parse(Path::new("sample.py"), source, 4).unwrap();

    assert_eq!(outline.language, Language::Python);
    assert_eq!(outline.nodes.len(), 2);
    assert_eq!(outline.nodes[0].label, "class Greeter");
    assert_eq!(outline.nodes[0].kind, NodeKind::Class);
    assert_eq!(outline.nodes[0].range, SourceRange::lines(1, 5));
    assert_eq!(outline.nodes[0].children[0].label, "def hello");
    assert_eq!(outline.nodes[0].children[0].range, SourceRange::lines(2, 5));
    assert_eq!(
        outline.nodes[0].children[0].children[0].label,
        "def format_name"
    );
    assert_eq!(
        outline.nodes[0].children[0].children[0].range,
        SourceRange::lines(3, 4)
    );
    assert_eq!(outline.nodes[1].label, "async def fetch");
    assert_eq!(outline.nodes[1].range, SourceRange::lines(7, 8));
}

#[test]
fn includes_decorators_in_python_ranges() {
    let source = "\
@classmethod
def build(cls):
    return cls()
";
    let outline = python::parse(Path::new("sample.py"), source, 3).unwrap();

    assert_eq!(outline.nodes[0].label, "def build");
    assert_eq!(outline.nodes[0].range, SourceRange::lines(1, 3));
}

#[test]
fn respects_max_depth_for_python_outline_depth() {
    let source = "\
class Greeter:
    def hello(self):
        def format_name():
            return 'Ada'
";
    let outline = python::parse(Path::new("sample.py"), source, 2).unwrap();

    assert_eq!(outline.nodes.len(), 1);
    assert_eq!(outline.nodes[0].children.len(), 1);
    assert!(outline.nodes[0].children[0].children.is_empty());
}

#[test]
fn reports_python_syntax_errors() {
    let error = python::parse(Path::new("bad.py"), "def broken(:\n", 3).unwrap_err();

    assert!(matches!(error, Error::Parse { .. }));
    assert!(error.to_string().contains("bad.py"));
}
