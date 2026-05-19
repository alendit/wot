use std::path::Path;

use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};

use crate::error::Result;
use crate::model::{Language, NodeKind, Outline, OutlineNode, SourceRange};
use crate::source_map::SourceMap;

pub fn parse(path: &Path, source: &str, max_depth: usize) -> Result<Outline> {
    let map = SourceMap::new(source);
    let mut starts = collect_heading_starts(source);

    for index in 0..starts.len() {
        let end = next_section_end(&starts, index, source.len());
        starts[index].range = map.range(starts[index].start_offset..end);
    }

    let mut index = 0;
    let nodes = build_nodes(&starts, &mut index, 0, 1, max_depth);

    Ok(Outline {
        path: path.to_path_buf(),
        language: Language::Markdown,
        nodes,
    })
}

#[derive(Debug, Clone)]
struct HeadingStart {
    level: usize,
    label: String,
    start_offset: usize,
    range: SourceRange,
}

fn collect_heading_starts(source: &str) -> Vec<HeadingStart> {
    let mut headings = Vec::new();
    let parser = Parser::new_ext(source, Options::all()).into_offset_iter();
    let mut current: Option<(usize, usize, String)> = None;

    for (event, range) in parser {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                current = Some((heading_level(level), range.start, String::new()));
            }
            Event::End(TagEnd::Heading(_)) => {
                if let Some((level, start_offset, label)) = current.take() {
                    headings.push(HeadingStart {
                        level,
                        label: normalize_label(&label),
                        start_offset,
                        range: SourceRange::lines(1, 1),
                    });
                }
            }
            Event::Text(text) | Event::Code(text) => {
                if let Some((_, _, label)) = current.as_mut() {
                    label.push_str(&text);
                }
            }
            Event::SoftBreak | Event::HardBreak => {
                if let Some((_, _, label)) = current.as_mut() {
                    label.push(' ');
                }
            }
            _ => {}
        }
    }

    headings
}

fn next_section_end(headings: &[HeadingStart], index: usize, source_len: usize) -> usize {
    let current_level = headings[index].level;
    headings[index + 1..]
        .iter()
        .find(|heading| heading.level <= current_level)
        .map(|heading| heading.start_offset)
        .unwrap_or(source_len)
}

fn build_nodes(
    headings: &[HeadingStart],
    index: &mut usize,
    parent_level: usize,
    depth: usize,
    max_depth: usize,
) -> Vec<OutlineNode> {
    let mut nodes = Vec::new();

    while let Some(heading) = headings.get(*index) {
        if heading.level <= parent_level {
            break;
        }

        *index += 1;
        let children = build_nodes(headings, index, heading.level, depth + 1, max_depth);

        if depth <= max_depth {
            nodes.push(OutlineNode {
                label: heading.label.clone(),
                kind: NodeKind::Heading,
                range: heading.range,
                children,
            });
        }
    }

    nodes
}

fn heading_level(level: HeadingLevel) -> usize {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

fn normalize_label(label: &str) -> String {
    label.split_whitespace().collect::<Vec<_>>().join(" ")
}
