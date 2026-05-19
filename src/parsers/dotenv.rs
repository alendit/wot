use std::path::Path;

use crate::error::Result;
use crate::model::{Language, NodeKind, Outline};
use crate::parsers::structured::{
    into_outline_nodes, is_secret_key, scalar_summary, strip_inline_comment, StructuredNode,
};

pub fn parse(path: &Path, source: &str, max_depth: usize) -> Result<Outline> {
    let nodes = source
        .lines()
        .enumerate()
        .filter_map(|(line_index, line)| parse_line(line, line_index + 1))
        .collect();

    Ok(Outline {
        path: path.to_path_buf(),
        language: Language::Dotenv,
        nodes: into_outline_nodes(nodes, 1, max_depth),
    })
}

fn parse_line(line: &str, line_number: usize) -> Option<StructuredNode> {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return None;
    }

    let assignment = trimmed.strip_prefix("export ").unwrap_or(trimmed);
    let (key, value) = assignment.split_once('=')?;
    let key = key.trim();
    let value = strip_inline_comment(value.trim());
    let summary = if is_secret_key(key) {
        "<redacted>".into()
    } else {
        scalar_summary(value)
    };

    Some(StructuredNode::new(
        format!("{key}: {summary}"),
        NodeKind::ConfigKey,
        line_number,
        line_number,
    ))
}
