use std::path::Path;

use tree_sitter::{Language as TreeSitterLanguage, Node, Parser};

use crate::error::{Error, Result};
use crate::model::{Language, NodeKind, Outline, OutlineNode, SourceRange};

pub fn parse(path: &Path, source: &str, language: Language, max_depth: usize) -> Result<Outline> {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_language(path, language))
        .map_err(|error| Error::Parse {
            path: path.to_path_buf(),
            message: error.to_string(),
        })?;
    let tree = parser.parse(source, None).ok_or_else(|| Error::Parse {
        path: path.to_path_buf(),
        message: "tree-sitter parser failed".into(),
    })?;

    Ok(Outline {
        path: path.to_path_buf(),
        language,
        nodes: extract_children(tree.root_node(), source, language, 1, max_depth),
    })
}

fn tree_sitter_language(path: &Path, language: Language) -> TreeSitterLanguage {
    match language {
        Language::Rust => tree_sitter_rust::LANGUAGE.into(),
        Language::TypeScript => {
            if path.extension().and_then(|extension| extension.to_str()) == Some("tsx") {
                tree_sitter_typescript::LANGUAGE_TSX.into()
            } else {
                tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()
            }
        }
        Language::JavaScript => tree_sitter_javascript::LANGUAGE.into(),
        Language::Go => tree_sitter_go::LANGUAGE.into(),
        Language::C => tree_sitter_c::LANGUAGE.into(),
        Language::Cpp => tree_sitter_cpp::LANGUAGE.into(),
        Language::Java => tree_sitter_java::LANGUAGE.into(),
        Language::Kotlin => tree_sitter_kotlin_ng::LANGUAGE.into(),
        Language::CSharp => tree_sitter_c_sharp::LANGUAGE.into(),
        Language::Shell => tree_sitter_bash::LANGUAGE.into(),
        Language::Clojure => tree_sitter_clojure_orchard::LANGUAGE.into(),
        Language::Elisp => tree_sitter_elisp::LANGUAGE.into(),
        _ => unreachable!("tree-sitter parser called for non-code language"),
    }
}

fn extract_children(
    node: Node<'_>,
    source: &str,
    language: Language,
    depth: usize,
    max_depth: usize,
) -> Vec<OutlineNode> {
    if depth > max_depth {
        return Vec::new();
    }

    let mut cursor = node.walk();
    let mut output = Vec::new();
    for child in node.named_children(&mut cursor) {
        if let Some((label, kind, include_children)) = classify_node(child, source, language) {
            let children = if include_children {
                extract_children(child, source, language, depth + 1, max_depth)
            } else {
                Vec::new()
            };
            output.push(OutlineNode {
                label,
                kind,
                range: node_range(child),
                children,
            });
        } else {
            output.extend(extract_children(child, source, language, depth, max_depth));
        }
    }
    output
}

fn classify_node(
    node: Node<'_>,
    source: &str,
    language: Language,
) -> Option<(String, NodeKind, bool)> {
    match language {
        Language::Rust => classify_rust(node, source),
        Language::TypeScript | Language::JavaScript => classify_ts_js(node, source),
        Language::Go => classify_go(node, source),
        Language::C | Language::Cpp => classify_c_cpp(node, source, language),
        Language::Java => classify_java_like(node, source, JavaLike::Java),
        Language::Kotlin => classify_java_like(node, source, JavaLike::Kotlin),
        Language::CSharp => classify_java_like(node, source, JavaLike::CSharp),
        Language::Shell => classify_shell(node, source),
        Language::Clojure => classify_clojure(node, source),
        Language::Elisp => classify_elisp(node, source),
        _ => None,
    }
}

fn classify_rust(node: Node<'_>, source: &str) -> Option<(String, NodeKind, bool)> {
    match node.kind() {
        "use_declaration" => Some((
            format!("use {}", rust_use_path(node, source)?),
            NodeKind::Import,
            false,
        )),
        "mod_item" => Some((
            format!("mod {}", name_text(node, source)?),
            NodeKind::Module,
            true,
        )),
        "struct_item" => Some((
            format!("struct {}", name_text(node, source)?),
            NodeKind::Type,
            true,
        )),
        "enum_item" => Some((
            format!("enum {}", name_text(node, source)?),
            NodeKind::Type,
            true,
        )),
        "trait_item" => Some((
            format!("trait {}", name_text(node, source)?),
            NodeKind::Type,
            true,
        )),
        "type_item" => Some((
            format!("type {}", name_text(node, source)?),
            NodeKind::Type,
            false,
        )),
        "impl_item" => Some((rust_impl_label(node, source), NodeKind::Type, true)),
        "function_item" => Some((
            format!("fn {}", name_text(node, source)?),
            NodeKind::Function,
            true,
        )),
        _ => None,
    }
}

fn classify_ts_js(node: Node<'_>, source: &str) -> Option<(String, NodeKind, bool)> {
    match node.kind() {
        "import_statement" => Some((js_import_label(node, source), NodeKind::Import, false)),
        "export_statement" => Some((js_export_label(node, source)?, NodeKind::Export, false)),
        "class_declaration" => Some((
            format!("class {}", name_text(node, source)?),
            NodeKind::Class,
            true,
        )),
        "interface_declaration" => Some((
            format!("interface {}", name_text(node, source)?),
            NodeKind::Type,
            true,
        )),
        "type_alias_declaration" => Some((
            format!("type {}", name_text(node, source)?),
            NodeKind::Type,
            false,
        )),
        "function_declaration" => Some((
            format!("function {}", name_text(node, source)?),
            NodeKind::Function,
            true,
        )),
        "method_definition" => Some((
            format!("method {}", name_text(node, source)?),
            NodeKind::Method,
            true,
        )),
        "lexical_declaration" | "variable_declaration" => {
            js_component_label(node, source).map(|label| (label, NodeKind::Component, true))
        }
        _ => None,
    }
}

fn classify_go(node: Node<'_>, source: &str) -> Option<(String, NodeKind, bool)> {
    match node.kind() {
        "import_declaration" => Some((
            single_line(node, source).trim_end_matches(';').to_string(),
            NodeKind::Import,
            false,
        )),
        "type_declaration" => Some((go_type_label(node, source)?, NodeKind::Type, true)),
        "function_declaration" => Some((
            format!("func {}", name_text(node, source)?),
            NodeKind::Function,
            true,
        )),
        "method_declaration" => Some((
            format!("method {}", name_text(node, source)?),
            NodeKind::Method,
            true,
        )),
        _ => None,
    }
}

fn classify_c_cpp(
    node: Node<'_>,
    source: &str,
    language: Language,
) -> Option<(String, NodeKind, bool)> {
    match node.kind() {
        "preproc_include" => Some((single_line(node, source), NodeKind::Import, false)),
        "namespace_definition" if language == Language::Cpp => Some((
            format!("namespace {}", name_text(node, source)?),
            NodeKind::Module,
            true,
        )),
        "class_specifier" if language == Language::Cpp => Some((
            format!("class {}", name_text(node, source)?),
            NodeKind::Class,
            true,
        )),
        "struct_specifier" => Some((
            format!("struct {}", name_text(node, source).unwrap_or_default())
                .trim()
                .to_string(),
            NodeKind::Type,
            true,
        )),
        "union_specifier" => Some((
            format!("union {}", name_text(node, source).unwrap_or_default())
                .trim()
                .to_string(),
            NodeKind::Type,
            true,
        )),
        "enum_specifier" => Some((
            format!("enum {}", name_text(node, source).unwrap_or_default())
                .trim()
                .to_string(),
            NodeKind::Type,
            true,
        )),
        "type_definition" => {
            c_type_definition_label(node, source).map(|label| (label, NodeKind::Type, true))
        }
        "alias_declaration" if language == Language::Cpp => {
            c_alias_label(node, source).map(|label| (label, NodeKind::Type, false))
        }
        "function_definition" => c_function_name(node, source).map(|name| {
            let kind = if language == Language::Cpp && name.contains("::") {
                NodeKind::Method
            } else {
                NodeKind::Function
            };
            (format!("function {name}"), kind, true)
        }),
        "field_declaration" if language == Language::Cpp => c_field_method_name(node, source)
            .map(|name| (format!("method {name}"), NodeKind::Method, false)),
        _ => None,
    }
}

#[derive(Debug, Clone, Copy)]
enum JavaLike {
    Java,
    Kotlin,
    CSharp,
}

fn classify_java_like(
    node: Node<'_>,
    source: &str,
    language: JavaLike,
) -> Option<(String, NodeKind, bool)> {
    match node.kind() {
        "class_declaration" => Some((
            format!("class {}", name_text(node, source)?),
            NodeKind::Class,
            true,
        )),
        "interface_declaration" => Some((
            format!("interface {}", name_text(node, source)?),
            NodeKind::Type,
            true,
        )),
        "enum_declaration" => Some((
            format!("enum {}", name_text(node, source)?),
            NodeKind::Type,
            true,
        )),
        "object_declaration" => Some((
            format!("object {}", name_text(node, source)?),
            NodeKind::Module,
            true,
        )),
        "method_declaration" => Some((
            format!("method {}", name_text(node, source)?),
            NodeKind::Method,
            true,
        )),
        "constructor_declaration" => Some((
            format!(
                "constructor {}",
                name_text(node, source).unwrap_or_default()
            ),
            NodeKind::Method,
            true,
        )),
        "function_declaration" if matches!(language, JavaLike::Kotlin) => Some((
            format!("fun {}", name_text(node, source)?),
            NodeKind::Function,
            true,
        )),
        _ => None,
    }
}

fn classify_shell(node: Node<'_>, source: &str) -> Option<(String, NodeKind, bool)> {
    match node.kind() {
        "function_definition" => Some((
            format!("function {}", name_text(node, source)?),
            NodeKind::Function,
            true,
        )),
        "case_statement" => Some((shell_case_label(node, source), NodeKind::Module, true)),
        _ => None,
    }
}

fn classify_clojure(node: Node<'_>, source: &str) -> Option<(String, NodeKind, bool)> {
    let text = node_text(node, source).trim();
    let (form, name) = lisp_form_name(text)?;
    match form {
        "ns" => Some((format!("ns {name}"), NodeKind::Module, false)),
        "defn" | "defn-" | "defmacro" | "defmethod" => {
            Some((format!("{form} {name}"), NodeKind::Function, true))
        }
        "defrecord" | "deftype" | "defprotocol" | "definterface" | "defmulti" => {
            Some((format!("{form} {name}"), NodeKind::Type, true))
        }
        "def" | "defonce" => Some((format!("{form} {name}"), NodeKind::ConfigKey, false)),
        _ => None,
    }
}

fn classify_elisp(node: Node<'_>, source: &str) -> Option<(String, NodeKind, bool)> {
    let text = node_text(node, source).trim();
    let (form, name) = lisp_form_name(text)?;
    match form {
        "require" => Some((
            format!("require {}", name.trim_start_matches('\'')),
            NodeKind::Import,
            false,
        )),
        "defun" | "defmacro" | "cl-defun" | "cl-defmacro" => {
            Some((format!("{form} {name}"), NodeKind::Function, true))
        }
        "defvar" | "defconst" | "setq" | "customize-set-variable" => {
            Some((format!("{form} {name}"), NodeKind::ConfigKey, false))
        }
        _ => None,
    }
}

fn node_range(node: Node<'_>) -> SourceRange {
    SourceRange::lines(node.start_position().row + 1, node.end_position().row + 1)
}

fn name_text(node: Node<'_>, source: &str) -> Option<String> {
    node.child_by_field_name("name")
        .map(|name| node_text(name, source).trim().to_string())
}

fn node_text<'a>(node: Node<'_>, source: &'a str) -> &'a str {
    &source[node.byte_range()]
}

fn single_line(node: Node<'_>, source: &str) -> String {
    node_text(node, source)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn rust_use_path(node: Node<'_>, source: &str) -> Option<String> {
    Some(
        single_line(node, source)
            .trim_start_matches("use ")
            .trim_end_matches(';')
            .to_string(),
    )
}

fn rust_impl_label(node: Node<'_>, source: &str) -> String {
    let declaration = single_line(node, source);
    let declaration = declaration.split('{').next().unwrap_or(&declaration).trim();
    strip_modifiers(declaration).to_string()
}

fn js_import_label(node: Node<'_>, source: &str) -> String {
    let text = single_line(node, source);
    let imported = text
        .trim_start_matches("import ")
        .split(" from ")
        .next()
        .unwrap_or("")
        .trim()
        .trim_matches('{')
        .trim_matches('}')
        .trim();
    if imported.is_empty() {
        "import".into()
    } else {
        format!("import {imported}")
    }
}

fn js_export_label(node: Node<'_>, source: &str) -> Option<String> {
    let text = single_line(node, source);
    let declaration = text
        .split('{')
        .next()
        .unwrap_or(&text)
        .trim_end_matches(';')
        .trim();
    let declaration = declaration
        .strip_prefix("export default ")
        .or_else(|| declaration.strip_prefix("export "))
        .unwrap_or(declaration);
    let normalized = strip_modifiers(declaration);

    if normalized.starts_with("interface ")
        || normalized.starts_with("type ")
        || normalized.starts_with("function ")
        || normalized.starts_with("class ")
        || normalized.starts_with("const ")
        || normalized.starts_with("let ")
        || normalized.starts_with("var ")
    {
        Some(format!("export {}", compact_js_declaration(normalized)))
    } else {
        Some("export".into())
    }
}

fn compact_js_declaration(declaration: &str) -> String {
    if let Some(rest) = declaration.strip_prefix("function ") {
        let name = rest.split('(').next().unwrap_or(rest).trim();
        format!("function {name}")
    } else if let Some(rest) = declaration.strip_prefix("class ") {
        let name = rest
            .split(|character: char| character.is_whitespace() || character == '{')
            .next()
            .unwrap_or(rest)
            .trim();
        format!("class {name}")
    } else if let Some(rest) = declaration.strip_prefix("interface ") {
        let name = rest
            .split(|character: char| character.is_whitespace() || character == '{')
            .next()
            .unwrap_or(rest)
            .trim();
        format!("interface {name}")
    } else if let Some(rest) = declaration.strip_prefix("type ") {
        let name = rest
            .split(|character: char| character.is_whitespace() || character == '=')
            .next()
            .unwrap_or(rest)
            .trim();
        format!("type {name}")
    } else {
        declaration.to_string()
    }
}

fn js_component_label(node: Node<'_>, source: &str) -> Option<String> {
    let text = node_text(node, source);
    let declarator = find_descendant_kind(node, "variable_declarator")?;
    let name = declarator.child_by_field_name("name")?;
    let name = node_text(name, source).trim();
    if !name.chars().next().is_some_and(char::is_uppercase) {
        return None;
    }
    if text.contains("=>") || text.contains("function") || text.contains('<') {
        Some(format!("component {name}"))
    } else {
        None
    }
}

fn go_type_label(node: Node<'_>, source: &str) -> Option<String> {
    let spec = find_descendant_kind(node, "type_spec")?;
    Some(format!("type {}", name_text(spec, source)?))
}

fn c_function_name(node: Node<'_>, source: &str) -> Option<String> {
    let declarator = node.child_by_field_name("declarator")?;
    c_declarator_name(declarator, source)
}

fn c_field_method_name(node: Node<'_>, source: &str) -> Option<String> {
    if !node_text(node, source).contains('(') {
        return None;
    }
    let declarator = node.child_by_field_name("declarator")?;
    c_declarator_name(declarator, source)
}

fn c_type_definition_label(node: Node<'_>, source: &str) -> Option<String> {
    let name = find_last_identifier(node, source)?;
    Some(format!("typedef {name}"))
}

fn c_alias_label(node: Node<'_>, source: &str) -> Option<String> {
    let name = find_last_identifier(node, source)?;
    Some(format!("alias {name}"))
}

fn c_declarator_name(node: Node<'_>, source: &str) -> Option<String> {
    match node.kind() {
        "identifier"
        | "field_identifier"
        | "qualified_identifier"
        | "operator_name"
        | "destructor_name" => Some(node_text(node, source).trim().to_string()),
        _ => {
            if let Some(declarator) = node.child_by_field_name("declarator") {
                return c_declarator_name(declarator, source);
            }
            if let Some(name) = node.child_by_field_name("name") {
                return c_declarator_name(name, source);
            }
            find_last_identifier(node, source)
        }
    }
}

fn find_last_identifier(node: Node<'_>, source: &str) -> Option<String> {
    let mut cursor = node.walk();
    let mut last = None;
    for child in node.named_children(&mut cursor) {
        if matches!(
            child.kind(),
            "identifier"
                | "field_identifier"
                | "qualified_identifier"
                | "operator_name"
                | "destructor_name"
        ) {
            last = Some(node_text(child, source).trim().to_string());
        }
        if let Some(descendant) = find_last_identifier(child, source) {
            last = Some(descendant);
        }
    }
    last
}

fn shell_case_label(node: Node<'_>, source: &str) -> String {
    let text = single_line(node, source);
    let subject = text
        .strip_prefix("case ")
        .and_then(|rest| rest.split(" in").next())
        .unwrap_or("")
        .trim();
    if subject.is_empty() {
        "case".into()
    } else {
        format!("case {subject}")
    }
}

fn lisp_form_name(text: &str) -> Option<(&str, &str)> {
    let text = text.strip_prefix('(')?;
    let mut parts = text.split_whitespace();
    let form = parts.next()?;
    let name = parts.next()?.trim_end_matches(')');
    Some((form, name))
}

fn find_descendant_kind<'tree>(node: Node<'tree>, kind: &str) -> Option<Node<'tree>> {
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if child.kind() == kind {
            return Some(child);
        }
        if let Some(descendant) = find_descendant_kind(child, kind) {
            return Some(descendant);
        }
    }
    None
}

fn strip_modifiers(mut declaration: &str) -> &str {
    loop {
        let trimmed = declaration.trim_start();
        let Some((first, rest)) = trimmed.split_once(char::is_whitespace) else {
            return trimmed;
        };
        if matches!(
            first,
            "pub"
                | "public"
                | "private"
                | "protected"
                | "internal"
                | "static"
                | "final"
                | "abstract"
                | "async"
        ) {
            declaration = rest;
        } else {
            return trimmed;
        }
    }
}
