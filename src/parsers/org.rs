use std::path::Path;

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
        language: Language::Org,
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
    let mut offset = 0;
    let mut in_block = false;

    for raw_line in source.split_inclusive('\n') {
        let line = raw_line.trim_end_matches(['\r', '\n']);
        let lower_trimmed = line.trim_start().to_ascii_lowercase();

        if lower_trimmed.starts_with("#+begin_") {
            in_block = true;
        } else if lower_trimmed.starts_with("#+end_") {
            in_block = false;
        } else if !in_block {
            if let Some((level, label)) = parse_heading_line(line) {
                headings.push(HeadingStart {
                    level,
                    label: normalize_label(label),
                    start_offset: offset,
                    range: SourceRange::lines(1, 1),
                });
            }
        }

        offset += raw_line.len();
    }

    headings
}

fn parse_heading_line(line: &str) -> Option<(usize, &str)> {
    let level = line.bytes().take_while(|byte| *byte == b'*').count();
    if level == 0 {
        return None;
    }

    let rest = &line[level..];
    if !rest.starts_with(char::is_whitespace) {
        return None;
    }

    Some((level, rest.trim()))
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

fn normalize_label(label: &str) -> String {
    label.split_whitespace().collect::<Vec<_>>().join(" ")
}
