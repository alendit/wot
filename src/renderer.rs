use serde::Serialize;

use crate::error::Error;
use crate::model::{Language, LanguageSpec, NodeKind, Outline, OutlineNode, Position, SourceRange};

#[derive(Debug, Clone, Serialize)]
struct JsonResponse {
    files: Vec<RenderedFile>,
    errors: Vec<RenderError>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "mode", rename_all = "lowercase")]
pub enum RenderedFile {
    Outline {
        path: String,
        language: String,
        truncated: bool,
        omitted_nodes: usize,
        nodes: Vec<JsonNode>,
    },
    Verbatim {
        path: String,
        language: String,
        content: String,
    },
}

impl RenderedFile {
    pub fn outline(outline: Outline, max_items: usize) -> Self {
        let total = count_outline_nodes(&outline.nodes);
        let (nodes, kept) = cap_nodes(&outline.nodes, max_items);
        let omitted_nodes = total.saturating_sub(kept);
        Self::Outline {
            path: outline.path.display().to_string(),
            language: outline.language.to_string(),
            truncated: omitted_nodes > 0,
            omitted_nodes,
            nodes,
        }
    }

    pub fn verbatim(path: std::path::PathBuf, language: Language, content: String) -> Self {
        Self::Verbatim {
            path: path.display().to_string(),
            language: language.to_string(),
            content,
        }
    }

    fn path(&self) -> &str {
        match self {
            Self::Outline { path, .. } | Self::Verbatim { path, .. } => path,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct JsonNode {
    label: String,
    kind: NodeKind,
    range: JsonRange,
    children: Vec<JsonNode>,
}

#[derive(Debug, Clone, Serialize)]
struct JsonRange {
    display: String,
    start: Position,
    end: Position,
    precise: bool,
}

impl From<SourceRange> for JsonRange {
    fn from(range: SourceRange) -> Self {
        Self {
            display: range.display(),
            start: range.start,
            end: range.end,
            precise: range.precise,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct RenderError {
    pub message: String,
}

impl From<Error> for RenderError {
    fn from(error: Error) -> Self {
        Self {
            message: error.to_string(),
        }
    }
}

pub fn render_markdown(files: &[RenderedFile], header: bool) -> String {
    let mut output = String::new();

    for (index, file) in files.iter().enumerate() {
        if index > 0 {
            output.push('\n');
        }

        match file {
            RenderedFile::Outline { nodes, .. } => {
                if header {
                    output.push_str("# ");
                    output.push_str(file.path());
                    output.push('\n');
                }
                render_nodes(&mut output, nodes, 0);
            }
            RenderedFile::Verbatim { content, .. } => {
                if header {
                    output.push_str("# ");
                    output.push_str(file.path());
                    output.push('\n');
                }
                output.push_str(content);
            }
        }
    }

    output
}

pub fn render_json(files: Vec<RenderedFile>, errors: Vec<RenderError>) -> String {
    let response = JsonResponse { files, errors };
    serde_json::to_string_pretty(&response).expect("serialize output")
}

pub fn render_supported_markdown(specs: &[LanguageSpec]) -> String {
    let mut output = String::new();
    for spec in specs {
        output.push_str("- ");
        output.push_str(&spec.language.to_string());
        output.push_str(": extensions [");
        output.push_str(&spec.extensions.join(", "));
        output.push_str("], filenames [");
        output.push_str(&spec.filenames.join(", "));
        output.push_str("], backend ");
        output.push_str(spec.backend);
        output.push('\n');
    }
    output
}

pub fn render_supported_json(specs: &[LanguageSpec]) -> String {
    let response = SupportedResponse {
        languages: specs.iter().map(SupportedLanguage::from).collect(),
    };
    serde_json::to_string_pretty(&response).expect("serialize supported languages")
}

fn render_nodes(output: &mut String, nodes: &[JsonNode], depth: usize) {
    let indent = "  ".repeat(depth);

    for node in nodes {
        output.push_str(&indent);
        output.push_str("- ");
        output.push_str(&node.label);
        output.push_str(" [");
        output.push_str(&node.range.display);
        output.push_str("]\n");

        render_nodes(output, &node.children, depth + 1);
    }
}

fn cap_nodes(nodes: &[OutlineNode], max_items: usize) -> (Vec<JsonNode>, usize) {
    let mut remaining = max_items;
    let nodes = cap_nodes_inner(nodes, &mut remaining);
    let kept = max_items.saturating_sub(remaining);
    (nodes, kept)
}

fn cap_nodes_inner(nodes: &[OutlineNode], remaining: &mut usize) -> Vec<JsonNode> {
    let mut output = Vec::new();
    for node in nodes {
        if *remaining == 0 {
            break;
        }
        *remaining -= 1;
        output.push(JsonNode {
            label: node.label.clone(),
            kind: node.kind,
            range: node.range.into(),
            children: cap_nodes_inner(&node.children, remaining),
        });
    }
    output
}

fn count_outline_nodes(nodes: &[OutlineNode]) -> usize {
    nodes
        .iter()
        .map(|node| 1 + count_outline_nodes(&node.children))
        .sum()
}

#[derive(Debug, Clone, Serialize)]
struct SupportedResponse {
    languages: Vec<SupportedLanguage>,
}

#[derive(Debug, Clone, Serialize)]
struct SupportedLanguage {
    id: String,
    names: &'static [&'static str],
    extensions: &'static [&'static str],
    filenames: &'static [&'static str],
    backend: &'static str,
}

impl From<&LanguageSpec> for SupportedLanguage {
    fn from(spec: &LanguageSpec) -> Self {
        Self {
            id: spec.language.to_string(),
            names: spec.names,
            extensions: spec.extensions,
            filenames: spec.filenames,
            backend: spec.backend,
        }
    }
}
