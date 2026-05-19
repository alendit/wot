use crate::model::{NodeKind, OutlineNode, SourceRange};

#[derive(Debug, Clone)]
pub struct StructuredNode {
    pub label: String,
    pub kind: NodeKind,
    pub start_line: usize,
    pub end_line: usize,
    pub children: Vec<StructuredNode>,
}

impl StructuredNode {
    pub fn new(
        label: impl Into<String>,
        kind: NodeKind,
        start_line: usize,
        end_line: usize,
    ) -> Self {
        Self {
            label: label.into(),
            kind,
            start_line,
            end_line,
            children: Vec::new(),
        }
    }
}

pub fn into_outline_nodes(
    nodes: Vec<StructuredNode>,
    depth: usize,
    max_depth: usize,
) -> Vec<OutlineNode> {
    if depth > max_depth {
        return Vec::new();
    }

    nodes
        .into_iter()
        .map(|node| OutlineNode {
            label: node.label,
            kind: node.kind,
            range: SourceRange::lines(node.start_line, node.end_line),
            children: into_outline_nodes(node.children, depth + 1, max_depth),
        })
        .collect()
}

pub fn scalar_summary(value: &str) -> String {
    let value = value.trim();
    if value.is_empty() {
        return "\"\"".into();
    }
    if is_quoted(value) {
        return format!("{:?}", truncate(trim_matching_quotes(value), 40));
    }
    let unquoted = trim_matching_quotes(value);
    if is_literal(unquoted) || is_number(unquoted) || looks_like_collection(value) {
        truncate(unquoted, 40)
    } else {
        format!("{:?}", truncate(unquoted, 40))
    }
}

pub fn trim_matching_quotes(value: &str) -> &str {
    let value = value.trim();
    if value.len() >= 2
        && ((value.starts_with('"') && value.ends_with('"'))
            || (value.starts_with('\'') && value.ends_with('\'')))
    {
        &value[1..value.len() - 1]
    } else {
        value
    }
}

pub fn is_secret_key(key: &str) -> bool {
    let upper = key.to_ascii_uppercase();
    [
        "SECRET",
        "TOKEN",
        "PASSWORD",
        "PASS",
        "API_KEY",
        "PRIVATE_KEY",
        "CREDENTIAL",
    ]
    .iter()
    .any(|needle| upper.contains(needle))
}

pub fn strip_inline_comment(value: &str) -> &str {
    let mut in_single = false;
    let mut in_double = false;

    for (index, character) in value.char_indices() {
        match character {
            '\'' if !in_double => in_single = !in_single,
            '"' if !in_single => in_double = !in_double,
            '#' | ';' if !in_single && !in_double => return value[..index].trim_end(),
            _ => {}
        }
    }

    value.trim_end()
}

fn is_quoted(value: &str) -> bool {
    value.len() >= 2
        && ((value.starts_with('"') && value.ends_with('"'))
            || (value.starts_with('\'') && value.ends_with('\'')))
}

fn is_literal(value: &str) -> bool {
    matches!(value, "true" | "false" | "null" | "nil")
}

fn is_number(value: &str) -> bool {
    value.parse::<f64>().is_ok()
}

fn looks_like_collection(value: &str) -> bool {
    (value.starts_with('[') && value.ends_with(']'))
        || (value.starts_with('{') && value.ends_with('}'))
}

fn truncate(value: &str, max_chars: usize) -> String {
    let mut output = String::new();
    for (index, character) in value.chars().enumerate() {
        if index == max_chars {
            output.push_str("...");
            break;
        }
        output.push(character);
    }
    output
}
