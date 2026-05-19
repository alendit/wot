use std::path::Path;

use crate::error::Result;
use crate::model::{Language, NodeKind, Outline};
use crate::parsers::structured::{into_outline_nodes, StructuredNode};

pub fn parse(path: &Path, source: &str, max_depth: usize) -> Result<Outline> {
    Ok(Outline {
        path: path.to_path_buf(),
        language: Language::Dockerfile,
        nodes: into_outline_nodes(collect_nodes(source), 1, max_depth),
    })
}

#[derive(Debug)]
struct Instruction {
    text: String,
    start_line: usize,
    end_line: usize,
}

fn collect_nodes(source: &str) -> Vec<StructuredNode> {
    let mut roots = Vec::new();
    let mut current_stage: Option<StructuredNode> = None;

    for instruction in logical_instructions(source) {
        let keyword = instruction
            .text
            .split_whitespace()
            .next()
            .unwrap_or("")
            .to_ascii_uppercase();

        if keyword == "FROM" {
            if let Some(stage) = current_stage.take() {
                roots.push(stage);
            }
            current_stage = Some(StructuredNode::new(
                instruction.text,
                NodeKind::DockerStage,
                instruction.start_line,
                instruction.end_line,
            ));
        } else {
            let node = StructuredNode::new(
                instruction.text,
                NodeKind::DockerInstruction,
                instruction.start_line,
                instruction.end_line,
            );
            if let Some(stage) = current_stage.as_mut() {
                stage.end_line = instruction.end_line;
                stage.children.push(node);
            } else {
                roots.push(node);
            }
        }
    }

    if let Some(stage) = current_stage {
        roots.push(stage);
    }

    roots
}

fn logical_instructions(source: &str) -> Vec<Instruction> {
    let mut instructions = Vec::new();
    let mut current: Option<Instruction> = None;

    for (line_index, line) in source.lines().enumerate() {
        let line_number = line_index + 1;
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        let continues = trimmed.ends_with('\\');
        let part = trimmed.trim_end_matches('\\').trim_end();
        if let Some(instruction) = current.as_mut() {
            if !instruction.text.is_empty() && !part.is_empty() {
                instruction.text.push(' ');
            }
            instruction.text.push_str(part.trim_start());
            instruction.end_line = line_number;
        } else {
            current = Some(Instruction {
                text: part.to_string(),
                start_line: line_number,
                end_line: line_number,
            });
        }

        if !continues {
            if let Some(instruction) = current.take() {
                instructions.push(instruction);
            }
        }
    }

    if let Some(instruction) = current {
        instructions.push(instruction);
    }

    instructions
}
