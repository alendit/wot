use std::path::Path;

use crate::error::{Error, Result};
use crate::model::{Language, NodeKind, Outline};
use crate::parsers::structured::{
    into_outline_nodes, scalar_summary, strip_inline_comment, StructuredNode,
};

pub fn parse(path: &Path, source: &str, max_depth: usize) -> Result<Outline> {
    parse_with_options(path, source, max_depth, false)
}

pub fn parse_with_options(
    path: &Path,
    source: &str,
    max_depth: usize,
    lenient: bool,
) -> Result<Outline> {
    if !lenient {
        toml::from_str::<toml::Value>(source).map_err(|error| Error::Parse {
            path: path.to_path_buf(),
            message: error.to_string(),
        })?;
    }

    Ok(Outline {
        path: path.to_path_buf(),
        language: Language::Toml,
        nodes: into_outline_nodes(collect_nodes(source), 1, max_depth),
    })
}

fn collect_nodes(source: &str) -> Vec<StructuredNode> {
    let mut roots = Vec::new();
    let mut current_section: Option<StructuredNode> = None;
    let mut previous_content_line = 1;

    for (line_index, line) in source.lines().enumerate() {
        let line_number = line_index + 1;
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            if let Some(mut section) = current_section.take() {
                section.end_line = previous_content_line;
                roots.push(section);
            }

            let is_array = trimmed.starts_with("[[") && trimmed.ends_with("]]");
            let name = if is_array {
                trimmed.trim_start_matches("[[").trim_end_matches("]]")
            } else {
                trimmed.trim_start_matches('[').trim_end_matches(']')
            };
            let label = if is_array {
                format!("{name}[]")
            } else {
                name.to_string()
            };
            current_section = Some(StructuredNode::new(
                label,
                NodeKind::ConfigSection,
                line_number,
                line_number,
            ));
            previous_content_line = line_number;
            continue;
        }

        if let Some((key, value)) = split_assignment(trimmed) {
            let node = StructuredNode::new(
                format!("{key}: {}", scalar_summary(value)),
                NodeKind::ConfigKey,
                line_number,
                line_number,
            );
            if let Some(section) = current_section.as_mut() {
                section.children.push(node);
            } else {
                roots.push(node);
            }
            previous_content_line = line_number;
        }
    }

    if let Some(mut section) = current_section {
        section.end_line = previous_content_line;
        roots.push(section);
    }

    roots
}

fn split_assignment(line: &str) -> Option<(&str, &str)> {
    let (key, value) = line.split_once('=')?;
    Some((key.trim(), strip_inline_comment(value.trim())))
}
