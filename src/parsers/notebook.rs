use std::path::Path;

use serde_json::Value;

use crate::error::{Error, Result};
use crate::model::{Language, NodeKind, Outline, OutlineNode, SourceRange};
use crate::parsers::{markdown, python};
use crate::source_map::SourceMap;

pub fn parse(path: &Path, source: &str, max_depth: usize) -> Result<Outline> {
    let value = serde_json::from_str::<Value>(source).map_err(|error| Error::Parse {
        path: path.to_path_buf(),
        message: error.to_string(),
    })?;
    let map = SourceMap::new(source);
    let cells = value
        .get("cells")
        .and_then(Value::as_array)
        .ok_or_else(|| Error::Parse {
            path: path.to_path_buf(),
            message: "notebook missing cells array".into(),
        })?;
    let cell_lines = find_cell_lines(source);

    let nodes = cells
        .iter()
        .enumerate()
        .filter_map(|(index, cell)| {
            let cell_type = cell.get("cell_type").and_then(Value::as_str)?;
            let cell_source = read_cell_source(cell.get("source")?);
            let start_line = cell_lines
                .get(index)
                .copied()
                .unwrap_or_else(|| map.line_count());
            Some(build_cell_node(
                cell_type,
                index + 1,
                &cell_source,
                start_line,
                max_depth,
            ))
        })
        .collect();

    Ok(Outline {
        path: path.to_path_buf(),
        language: Language::Notebook,
        nodes,
    })
}

fn build_cell_node(
    cell_type: &str,
    cell_number: usize,
    source: &str,
    start_line: usize,
    max_depth: usize,
) -> OutlineNode {
    let label = format!("{cell_type} cell {cell_number}");
    let children = match cell_type {
        "markdown" => markdown::parse(Path::new("cell.md"), source, max_depth)
            .map(|outline| outline.nodes)
            .unwrap_or_default(),
        "code" => python::parse(Path::new("cell.py"), source, max_depth)
            .map(|outline| outline.nodes)
            .unwrap_or_default(),
        _ => Vec::new(),
    };
    let end_line = start_line + source.lines().count().saturating_sub(1);

    OutlineNode {
        label,
        kind: NodeKind::NotebookCell,
        range: SourceRange::lines(start_line, end_line.max(start_line)),
        children: offset_nodes(children, start_line.saturating_sub(1)),
    }
}

fn read_cell_source(value: &Value) -> String {
    if let Some(source) = value.as_str() {
        source.to_string()
    } else if let Some(lines) = value.as_array() {
        lines.iter().filter_map(Value::as_str).collect()
    } else {
        String::new()
    }
}

fn find_cell_lines(source: &str) -> Vec<usize> {
    source
        .lines()
        .enumerate()
        .filter_map(|(index, line)| line.contains("\"cell_type\"").then_some(index + 1))
        .collect()
}

fn offset_nodes(mut nodes: Vec<OutlineNode>, offset: usize) -> Vec<OutlineNode> {
    for node in &mut nodes {
        node.range.start.line += offset;
        node.range.end.line += offset;
        node.children = offset_nodes(std::mem::take(&mut node.children), offset);
    }
    nodes
}
