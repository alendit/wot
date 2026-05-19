use std::collections::HashMap;
use std::path::Path;

use crate::error::{Error, Result};
use crate::model::{Language, NodeKind, Outline};
use crate::parsers::structured::{into_outline_nodes, scalar_summary, StructuredNode};

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
        serde_yaml::from_str::<serde_yaml::Value>(source).map_err(|error| Error::Parse {
            path: path.to_path_buf(),
            message: error.to_string(),
        })?;
    }

    let entries = collect_entries(source);
    let mut index = 0;
    let nodes = build_nodes(&entries, &mut index, -1, source.lines().count().max(1));

    Ok(Outline {
        path: path.to_path_buf(),
        language: Language::Yaml,
        nodes: into_outline_nodes(nodes, 1, max_depth),
    })
}

#[derive(Debug, Clone)]
struct Entry {
    indent: isize,
    label: String,
    kind: NodeKind,
    line: usize,
}

fn collect_entries(source: &str) -> Vec<Entry> {
    let mut entries = Vec::new();
    let mut array_counts = HashMap::<isize, usize>::new();

    for (line_index, line) in source.lines().enumerate() {
        let line_number = line_index + 1;
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed == "---" || trimmed == "..." {
            continue;
        }

        let indent = line
            .chars()
            .take_while(|character| *character == ' ')
            .count() as isize;
        array_counts.retain(|array_indent, _| *array_indent <= indent);

        if let Some(rest) = trimmed.strip_prefix("- ") {
            let count = array_counts.entry(indent).or_default();
            let label = format!("[{}]: {}", *count, yaml_value_summary(rest));
            *count += 1;
            entries.push(Entry {
                indent,
                label,
                kind: NodeKind::ConfigArrayElement,
                line: line_number,
            });
        } else if let Some((key, value)) = split_key_value(trimmed) {
            entries.push(Entry {
                indent,
                label: format!("{}: {}", clean_key(key), yaml_value_summary(value)),
                kind: NodeKind::ConfigKey,
                line: line_number,
            });
        }
    }

    entries
}

fn build_nodes(
    entries: &[Entry],
    index: &mut usize,
    parent_indent: isize,
    total_lines: usize,
) -> Vec<StructuredNode> {
    let mut nodes = Vec::new();

    while let Some(entry) = entries.get(*index) {
        if entry.indent <= parent_indent {
            break;
        }

        let current_index = *index;
        *index += 1;
        let children = build_nodes(entries, index, entry.indent, total_lines);
        let end_line = subtree_end_line(entries, current_index, total_lines);
        let mut label = entry.label.clone();
        if label.ends_with(": object")
            && !children.is_empty()
            && children
                .iter()
                .all(|child| child.kind == NodeKind::ConfigArrayElement)
        {
            let key = label.trim_end_matches(": object");
            label = format!("{key}: array[{}]", children.len());
        }

        nodes.push(StructuredNode {
            label,
            kind: entry.kind,
            start_line: entry.line,
            end_line,
            children,
        });
    }

    nodes
}

fn subtree_end_line(entries: &[Entry], index: usize, total_lines: usize) -> usize {
    let indent = entries[index].indent;
    entries[index + 1..]
        .iter()
        .find(|entry| entry.indent <= indent)
        .map(|entry| entry.line.saturating_sub(1).max(entries[index].line))
        .unwrap_or(total_lines)
}

fn split_key_value(line: &str) -> Option<(&str, &str)> {
    let (key, value) = line.split_once(':')?;
    Some((key.trim(), value.trim()))
}

fn yaml_value_summary(value: &str) -> String {
    let value = value.trim();
    if value.is_empty() {
        "object".into()
    } else {
        scalar_summary(value)
    }
}

fn clean_key(key: &str) -> String {
    key.trim_matches('"').trim_matches('\'').to_string()
}
