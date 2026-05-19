use std::path::Path;

use crate::error::{Error, Result};
use crate::model::{Language, NodeKind, Outline, OutlineNode};
use crate::source_map::SourceMap;

pub fn parse(path: &Path, source: &str, max_depth: usize) -> Result<Outline> {
    parse_with_options(path, source, max_depth, false)
}

pub fn parse_with_options(
    path: &Path,
    source: &str,
    max_depth: usize,
    lenient: bool,
) -> Result<Outline> {
    let scanner = XmlScanner::new(source);
    let nodes = scanner.parse(lenient).map_err(|message| Error::Parse {
        path: path.to_path_buf(),
        message,
    })?;

    Ok(Outline {
        path: path.to_path_buf(),
        language: Language::Xml,
        nodes: limit_depth(nodes, 1, max_depth),
    })
}

struct XmlScanner<'a> {
    source: &'a str,
    map: SourceMap<'a>,
}

#[derive(Debug)]
struct OpenElement {
    name: String,
    label: String,
    start: usize,
    children: Vec<OutlineNode>,
}

impl<'a> XmlScanner<'a> {
    fn new(source: &'a str) -> Self {
        Self {
            source,
            map: SourceMap::new(source),
        }
    }

    fn parse(self, lenient: bool) -> std::result::Result<Vec<OutlineNode>, String> {
        let mut roots = Vec::new();
        let mut stack = Vec::<OpenElement>::new();
        let mut position = 0;

        while let Some(relative_start) = self.source[position..].find('<') {
            let start = position + relative_start;
            let end = find_tag_end(self.source, start)
                .ok_or_else(|| "unterminated XML tag".to_string())?;
            let raw = self.source[start + 1..end].trim();
            position = end + 1;

            if raw.is_empty()
                || raw.starts_with('?')
                || raw.starts_with("!--")
                || raw.starts_with('!')
            {
                continue;
            }

            if let Some(name) = raw.strip_prefix('/') {
                let name = name.trim();
                let element = stack
                    .pop()
                    .ok_or_else(|| format!("unexpected closing tag `{name}`"))?;
                if element.name != name {
                    return Err(format!(
                        "closing tag `{name}` does not match `{}`",
                        element.name
                    ));
                }
                let node = OutlineNode {
                    label: element.label,
                    kind: NodeKind::XmlElement,
                    range: self.map.range(element.start..end + 1),
                    children: element.children,
                };
                attach_node(&mut roots, &mut stack, node);
                continue;
            }

            let self_closing = raw.ends_with('/');
            let content = raw.trim_end_matches('/').trim();
            let (name, label) = xml_label(content)?;
            if self_closing {
                attach_node(
                    &mut roots,
                    &mut stack,
                    OutlineNode {
                        label,
                        kind: NodeKind::XmlElement,
                        range: self.map.range(start..end + 1),
                        children: Vec::new(),
                    },
                );
            } else {
                stack.push(OpenElement {
                    name,
                    label,
                    start,
                    children: Vec::new(),
                });
            }
        }

        if lenient {
            while let Some(element) = stack.pop() {
                let end = self.source.len();
                let node = OutlineNode {
                    label: element.label,
                    kind: NodeKind::XmlElement,
                    range: self.map.range(element.start..end),
                    children: element.children,
                };
                attach_node(&mut roots, &mut stack, node);
            }
            Ok(roots)
        } else if let Some(element) = stack.last() {
            Err(format!("unclosed XML tag `{}`", element.name))
        } else {
            Ok(roots)
        }
    }
}

fn attach_node(roots: &mut Vec<OutlineNode>, stack: &mut [OpenElement], node: OutlineNode) {
    if let Some(parent) = stack.last_mut() {
        parent.children.push(node);
    } else {
        roots.push(node);
    }
}

fn find_tag_end(source: &str, start: usize) -> Option<usize> {
    let mut quote = None;
    for (offset, character) in source[start + 1..].char_indices() {
        match character {
            '\'' | '"' if quote == Some(character) => quote = None,
            '\'' | '"' if quote.is_none() => quote = Some(character),
            '>' if quote.is_none() => return Some(start + 1 + offset),
            _ => {}
        }
    }
    None
}

fn xml_label(content: &str) -> std::result::Result<(String, String), String> {
    let mut parts = content.splitn(2, char::is_whitespace);
    let name = parts
        .next()
        .filter(|name| !name.is_empty())
        .ok_or_else(|| "missing XML element name".to_string())?;
    let attrs = parts.next().unwrap_or("").trim();
    let label = if attrs.is_empty() {
        name.to_string()
    } else {
        format!("{name} {}", truncate_attrs(attrs))
    };
    Ok((name.to_string(), label))
}

fn truncate_attrs(attrs: &str) -> String {
    const MAX: usize = 80;
    if attrs.chars().count() <= MAX {
        attrs.to_string()
    } else {
        attrs.chars().take(MAX).collect::<String>() + "..."
    }
}

fn limit_depth(nodes: Vec<OutlineNode>, depth: usize, max_depth: usize) -> Vec<OutlineNode> {
    if depth > max_depth {
        return Vec::new();
    }

    nodes
        .into_iter()
        .map(|mut node| {
            node.children = limit_depth(node.children, depth + 1, max_depth);
            node
        })
        .collect()
}
