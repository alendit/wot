use std::path::Path;

use wot::model::{Language, NodeKind, SourceRange};
use wot::parsers;

#[test]
fn outlines_rust_modules_types_functions_and_methods() {
    let source = r#"use std::fmt;

mod inner {
    struct Thing;

    impl Thing {
        fn run(&self) {}
    }
}

pub fn top() {}
"#;
    let outline = parsers::parse_file(Path::new("lib.rs"), source, 4).unwrap();

    assert_eq!(outline.language, Language::Rust);
    assert_eq!(outline.nodes[0].label, "use std::fmt");
    assert_eq!(outline.nodes[0].kind, NodeKind::Import);
    assert_eq!(outline.nodes[1].label, "mod inner");
    assert_eq!(outline.nodes[1].kind, NodeKind::Module);
    assert_eq!(outline.nodes[1].range, SourceRange::lines(3, 9));
    assert_eq!(outline.nodes[1].children[0].label, "struct Thing");
    assert_eq!(outline.nodes[1].children[1].label, "impl Thing");
    assert_eq!(outline.nodes[1].children[1].children[0].label, "fn run");
    assert_eq!(outline.nodes[2].label, "fn top");
}

#[test]
fn outlines_typescript_imports_exports_classes_functions_and_components() {
    let source = r#"import React from "react";
export interface Props { name: string }
export function helper() { return 1; }
const Card = () => <div />;
class Service {
  run() {}
}
"#;
    let outline = parsers::parse_file(Path::new("component.tsx"), source, 3).unwrap();

    assert_eq!(outline.language, Language::TypeScript);
    assert_eq!(outline.nodes[0].label, "import React");
    assert_eq!(outline.nodes[0].kind, NodeKind::Import);
    assert_eq!(outline.nodes[1].label, "export interface Props");
    assert_eq!(outline.nodes[1].kind, NodeKind::Export);
    assert_eq!(outline.nodes[2].label, "export function helper");
    assert_eq!(outline.nodes[3].label, "component Card");
    assert_eq!(outline.nodes[3].kind, NodeKind::Component);
    assert_eq!(outline.nodes[4].label, "class Service");
    assert_eq!(outline.nodes[4].children[0].label, "method run");
}

#[test]
fn outlines_go_declarations() {
    let source = r#"package main

import "fmt"

type Server struct {}

func (s Server) Run() {}

func main() {}
"#;
    let outline = parsers::parse_file(Path::new("main.go"), source, 3).unwrap();

    assert_eq!(outline.language, Language::Go);
    assert_eq!(outline.nodes[0].label, "import \"fmt\"");
    assert_eq!(outline.nodes[1].label, "type Server");
    assert_eq!(outline.nodes[2].label, "method Run");
    assert_eq!(outline.nodes[3].label, "func main");
}

#[test]
fn outlines_c_includes_types_and_functions() {
    let source = r#"#include <stdio.h>

struct Point {
  int x;
};

int main(void) {
  return 0;
}
"#;
    let outline = parsers::parse_file(Path::new("main.c"), source, 3).unwrap();

    assert_eq!(outline.language, Language::C);
    assert_eq!(outline.nodes[0].label, "#include <stdio.h>");
    assert_eq!(outline.nodes[0].kind, NodeKind::Import);
    assert_eq!(outline.nodes[1].label, "struct Point");
    assert_eq!(outline.nodes[1].kind, NodeKind::Type);
    assert_eq!(outline.nodes[2].label, "function main");
    assert_eq!(outline.nodes[2].kind, NodeKind::Function);
}

#[test]
fn outlines_cpp_namespaces_classes_methods_and_functions() {
    let source = r#"#include <vector>

namespace demo {
class App {
public:
  void run();
};

void run() {}
}
"#;
    let outline = parsers::parse_file(Path::new("app.cpp"), source, 4).unwrap();

    assert_eq!(outline.language, Language::Cpp);
    assert_eq!(outline.nodes[0].label, "#include <vector>");
    assert_eq!(outline.nodes[0].kind, NodeKind::Import);
    assert_eq!(outline.nodes[1].label, "namespace demo");
    assert_eq!(outline.nodes[1].kind, NodeKind::Module);
    assert_eq!(outline.nodes[1].children[0].label, "class App");
    assert_eq!(outline.nodes[1].children[0].kind, NodeKind::Class);
    assert_eq!(outline.nodes[1].children[0].children[0].label, "method run");
    assert_eq!(outline.nodes[1].children[1].label, "function run");
}

#[test]
fn outlines_java_kotlin_and_csharp_class_members() {
    let java = "class App {\n  void run() {}\n}\n";
    let kotlin = "class App {\n  fun run() {}\n}\n";
    let csharp = "class App {\n  void Run() {}\n}\n";

    let java_outline = parsers::parse_file(Path::new("App.java"), java, 3).unwrap();
    let kotlin_outline = parsers::parse_file(Path::new("App.kt"), kotlin, 3).unwrap();
    let csharp_outline = parsers::parse_file(Path::new("App.cs"), csharp, 3).unwrap();

    assert_eq!(java_outline.nodes[0].label, "class App");
    assert_eq!(java_outline.nodes[0].children[0].label, "method run");
    assert_eq!(kotlin_outline.nodes[0].label, "class App");
    assert_eq!(kotlin_outline.nodes[0].children[0].label, "fun run");
    assert_eq!(csharp_outline.nodes[0].label, "class App");
    assert_eq!(csharp_outline.nodes[0].children[0].label, "method Run");
}

#[test]
fn outlines_shell_functions_and_case_blocks() {
    let source = r#"build() {
  echo build
}

case "$1" in
  start) build ;;
esac
"#;
    let outline = parsers::parse_file(Path::new("build.sh"), source, 3).unwrap();

    assert_eq!(outline.language, Language::Shell);
    assert_eq!(outline.nodes[0].label, "function build");
    assert_eq!(outline.nodes[0].kind, NodeKind::Function);
    assert_eq!(outline.nodes[1].label, "case \"$1\"");
    assert_eq!(outline.nodes[1].kind, NodeKind::Module);
}

#[test]
fn outlines_clojure_definitions() {
    let source = r#"(ns demo.core)

(defrecord User [name])

(defn greet [user]
  (:name user))
"#;
    let outline = parsers::parse_file(Path::new("core.clj"), source, 3).unwrap();

    assert_eq!(outline.language, Language::Clojure);
    assert_eq!(outline.nodes[0].label, "ns demo.core");
    assert_eq!(outline.nodes[0].kind, NodeKind::Module);
    assert_eq!(outline.nodes[1].label, "defrecord User");
    assert_eq!(outline.nodes[1].kind, NodeKind::Type);
    assert_eq!(outline.nodes[2].label, "defn greet");
}

#[test]
fn outlines_elisp_definitions() {
    let source = r#"(require 'cl-lib)

(defvar demo-name "wot")

(defun demo-run ()
  demo-name)
"#;
    let outline = parsers::parse_file(Path::new("demo.el"), source, 3).unwrap();

    assert_eq!(outline.language, Language::Elisp);
    assert_eq!(outline.nodes[0].label, "require cl-lib");
    assert_eq!(outline.nodes[0].kind, NodeKind::Import);
    assert_eq!(outline.nodes[1].label, "defvar demo-name");
    assert_eq!(outline.nodes[1].kind, NodeKind::ConfigKey);
    assert_eq!(outline.nodes[2].label, "defun demo-run");
}

#[test]
fn respects_max_depth_for_tree_sitter_outlines() {
    let source = "class App {\n  void run() {}\n}\n";
    let outline = parsers::parse_file(Path::new("App.java"), source, 1).unwrap();

    assert_eq!(outline.nodes[0].label, "class App");
    assert!(outline.nodes[0].children.is_empty());
}
