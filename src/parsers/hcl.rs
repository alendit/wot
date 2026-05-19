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
    let nodes = collect_nodes(source, lenient).map_err(|message| Error::Parse {
        path: path.to_path_buf(),
        message,
    })?;

    Ok(Outline {
        path: path.to_path_buf(),
        language: Language::Hcl,
        nodes: into_outline_nodes(nodes, 1, max_depth),
    })
}

fn collect_nodes(source: &str, lenient: bool) -> std::result::Result<Vec<StructuredNode>, String> {
    let mut roots = Vec::new();
    let mut stack = Vec::<StructuredNode>::new();
    let mut last_line = 1;

    for (line_index, line) in source.lines().enumerate() {
        let line_number = line_index + 1;
        last_line = line_number;
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with("//") {
            continue;
        }

        if trimmed.starts_with('}') {
            let mut node = stack
                .pop()
                .ok_or_else(|| "unexpected closing brace".to_string())?;
            node.end_line = line_number;
            attach_node(&mut roots, &mut stack, node);
            continue;
        }

        if let Some(label) = block_label(trimmed) {
            stack.push(StructuredNode::new(
                label,
                NodeKind::HclBlock,
                line_number,
                line_number,
            ));
            if trimmed.contains('}') {
                let mut node = stack.pop().expect("just pushed block");
                node.end_line = line_number;
                attach_node(&mut roots, &mut stack, node);
            }
            continue;
        }

        if let Some((key, value)) = trimmed.split_once('=') {
            let node = StructuredNode::new(
                format!(
                    "{}: {}",
                    key.trim(),
                    scalar_summary(strip_inline_comment(value))
                ),
                NodeKind::ConfigKey,
                line_number,
                line_number,
            );
            attach_node(&mut roots, &mut stack, node);
        }
    }

    if lenient {
        while let Some(mut node) = stack.pop() {
            node.end_line = last_line;
            attach_node(&mut roots, &mut stack, node);
        }
        Ok(roots)
    } else if let Some(node) = stack.last() {
        Err(format!("unclosed HCL block `{}`", node.label))
    } else {
        Ok(roots)
    }
}

fn block_label(line: &str) -> Option<String> {
    let before_brace = line.split_once('{')?.0.trim();
    if before_brace.is_empty() || before_brace.contains('=') {
        return None;
    }
    Some(before_brace.to_string())
}

fn attach_node(
    roots: &mut Vec<StructuredNode>,
    stack: &mut [StructuredNode],
    node: StructuredNode,
) {
    if let Some(parent) = stack.last_mut() {
        parent.children.push(node);
    } else {
        roots.push(node);
    }
}
