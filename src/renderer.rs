use serde::Serialize;

use crate::error::Error;
use crate::model::{Language, LanguageSpec, NodeKind, Outline, OutlineNode, Position, SourceRange};

#[derive(Debug, Clone, Serialize)]
struct JsonResponse {
    files: Vec<RenderedFile>,
    directories: Vec<JsonDirectoryRoot>,
    errors: Vec<RenderError>,
}

#[derive(Debug, Clone)]
pub(crate) enum RenderedRoot {
    File(RenderedFile),
    Directory(RenderedDirectory),
}

#[derive(Debug, Clone)]
pub(crate) struct RenderedDirectory {
    pub path: String,
    pub walk_depth: usize,
    pub depth_limited: bool,
    pub entries: Vec<RenderedDirectoryEntry>,
}

#[derive(Debug, Clone)]
pub(crate) enum RenderedDirectoryEntry {
    Directory(RenderedDirectory),
    File(RenderedFile),
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

    pub(crate) fn path(&self) -> &str {
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
        render_file_markdown(&mut output, file, header);
    }

    output
}

pub(crate) fn render_roots_markdown(roots: &[RenderedRoot], header: bool) -> String {
    let mut output = String::new();

    for (index, root) in roots.iter().enumerate() {
        if index > 0 {
            output.push('\n');
        }

        match root {
            RenderedRoot::File(file) => render_file_markdown(&mut output, file, header),
            RenderedRoot::Directory(directory) => {
                render_directory_markdown(&mut output, directory, 0, true)
            }
        }
    }

    output
}

pub fn render_json(files: Vec<RenderedFile>, errors: Vec<RenderError>) -> String {
    let response = JsonResponse {
        files,
        directories: Vec::new(),
        errors,
    };
    serde_json::to_string_pretty(&response).expect("serialize output")
}

pub(crate) fn render_roots_json(roots: &[RenderedRoot], errors: Vec<RenderError>) -> String {
    let mut files = Vec::new();
    let mut directories = Vec::new();

    for root in roots {
        match root {
            RenderedRoot::File(file) => files.push(file.clone()),
            RenderedRoot::Directory(directory) => {
                collect_directory_files(directory, &mut files);
                directories.push(JsonDirectoryRoot::from(directory));
            }
        }
    }

    let response = JsonResponse {
        files,
        directories,
        errors,
    };
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

fn render_file_markdown(output: &mut String, file: &RenderedFile, header: bool) {
    match file {
        RenderedFile::Outline { nodes, .. } => {
            if header {
                output.push_str("# ");
                output.push_str(file.path());
                output.push('\n');
            }
            render_nodes(output, nodes, 0);
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

fn render_directory_markdown(
    output: &mut String,
    directory: &RenderedDirectory,
    depth: usize,
    is_root: bool,
) {
    let indent = "  ".repeat(depth);
    let label = if is_root {
        directory.path.as_str()
    } else {
        display_name(&directory.path)
    };
    output.push_str(&indent);
    output.push_str("- ");
    output.push_str(&markdown_code_span(&directory_label(label)));
    if directory.depth_limited {
        output.push_str(&format!(
            " *(not expanded: walk depth limit {})*",
            directory.walk_depth
        ));
    }
    output.push('\n');

    if directory.depth_limited {
        return;
    }
    if directory.entries.is_empty() {
        output.push_str(&"  ".repeat(depth + 1));
        output.push_str("- *(no supported files)*\n");
        return;
    }

    for entry in &directory.entries {
        match entry {
            RenderedDirectoryEntry::Directory(child) => {
                render_directory_markdown(output, child, depth + 1, false)
            }
            RenderedDirectoryEntry::File(file) => {
                render_tree_file_markdown(output, file, depth + 1)
            }
        }
    }
}

fn render_tree_file_markdown(output: &mut String, file: &RenderedFile, depth: usize) {
    let indent = "  ".repeat(depth);
    output.push_str(&indent);
    output.push_str("- ");
    output.push_str(&markdown_code_span(display_name(file.path())));
    output.push('\n');

    match file {
        RenderedFile::Outline { nodes, .. } => render_nodes(output, nodes, depth + 1),
        RenderedFile::Verbatim {
            language, content, ..
        } => render_verbatim_block(output, language, content, depth + 1),
    }
}

fn render_verbatim_block(output: &mut String, language: &str, content: &str, depth: usize) {
    let indent = "  ".repeat(depth);
    let fence = markdown_fence(content);
    output.push_str(&indent);
    output.push_str(&fence);
    output.push_str(language);
    output.push('\n');
    for line in content.split_inclusive('\n') {
        output.push_str(&indent);
        output.push_str(line);
        if !line.ends_with('\n') {
            output.push('\n');
        }
    }
    output.push_str(&indent);
    output.push_str(&fence);
    output.push('\n');
}

fn markdown_fence(content: &str) -> String {
    "`".repeat(longest_backtick_run(content).saturating_add(1).max(3))
}

fn markdown_code_span(value: &str) -> String {
    let delimiter = "`".repeat(longest_backtick_run(value).saturating_add(1).max(1));
    if value.contains('`') {
        format!("{delimiter} {value} {delimiter}")
    } else {
        format!("{delimiter}{value}{delimiter}")
    }
}

fn directory_label(value: &str) -> String {
    if value.ends_with('/') || value.ends_with('\\') {
        value.to_owned()
    } else {
        format!("{value}{}", std::path::MAIN_SEPARATOR)
    }
}

fn longest_backtick_run(value: &str) -> usize {
    value
        .split(|character| character != '`')
        .map(str::len)
        .max()
        .unwrap_or(0)
}

fn display_name(path: &str) -> &str {
    std::path::Path::new(path)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(path)
}

fn collect_directory_files(directory: &RenderedDirectory, files: &mut Vec<RenderedFile>) {
    for entry in &directory.entries {
        match entry {
            RenderedDirectoryEntry::Directory(child) => collect_directory_files(child, files),
            RenderedDirectoryEntry::File(file) => files.push(file.clone()),
        }
    }
}

impl RenderedDirectory {
    fn truncated(&self) -> bool {
        self.depth_limited
            || self.entries.iter().any(|entry| match entry {
                RenderedDirectoryEntry::Directory(child) => child.truncated(),
                RenderedDirectoryEntry::File(_) => false,
            })
    }
}

#[derive(Debug, Clone, Serialize)]
struct JsonDirectoryRoot {
    path: String,
    max_depth: usize,
    truncated: bool,
    entries: Vec<JsonDirectoryEntry>,
}

impl From<&RenderedDirectory> for JsonDirectoryRoot {
    fn from(directory: &RenderedDirectory) -> Self {
        Self {
            path: directory.path.clone(),
            max_depth: directory.walk_depth,
            truncated: directory.truncated(),
            entries: directory
                .entries
                .iter()
                .map(JsonDirectoryEntry::from)
                .collect(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
enum JsonDirectoryEntry {
    Directory {
        path: String,
        truncated: bool,
        entries: Vec<JsonDirectoryEntry>,
    },
    File {
        path: String,
    },
}

impl From<&RenderedDirectoryEntry> for JsonDirectoryEntry {
    fn from(entry: &RenderedDirectoryEntry) -> Self {
        match entry {
            RenderedDirectoryEntry::Directory(directory) => Self::Directory {
                path: directory.path.clone(),
                truncated: directory.truncated(),
                entries: directory
                    .entries
                    .iter()
                    .map(JsonDirectoryEntry::from)
                    .collect(),
            },
            RenderedDirectoryEntry::File(file) => Self::File {
                path: file.path().to_owned(),
            },
        }
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
