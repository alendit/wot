use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use clap::{Parser, ValueEnum};
use serde::Serialize;

use crate::error::{Error, Result};
use crate::model::{Language, LanguageSpec, NodeKind, Outline, OutlineNode, Position, SourceRange};
use crate::parsers;

const DEFAULT_MAX_DEPTH: usize = 3;
const DEFAULT_MAX_ITEMS: usize = 200;
const DEFAULT_MIN_LINES: usize = 80;

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum OutputFormat {
    Markdown,
    Json,
}

#[derive(Debug, Parser)]
#[command(
    name = "wot",
    about = "Create compact outlines from source, config, and document files",
    long_about = "Create compact Markdown table-of-contents style outlines from source, config, and document files.\n\nSupported inputs include Rust, TypeScript/JavaScript, Go, C/C++, Java, Kotlin, C#, shell, Clojure, Emacs Lisp, Markdown, Python, JSON, YAML, TOML, INI, .env, XML/SVG/plist, HCL/Terraform, Dockerfile/Containerfile, and Jupyter notebooks.\n\nRanges are 1-based inclusive line ranges. When line-only ranges would be ambiguous, wot prints 1-based start-inclusive/end-exclusive columns as Lx:Cy-Lx:Cz."
)]
struct Args {
    #[arg(long, default_value_t = DEFAULT_MAX_DEPTH)]
    max_depth: usize,

    #[arg(long, value_enum, default_value_t = OutputFormat::Markdown)]
    format: OutputFormat,

    #[arg(long)]
    header: bool,

    #[arg(long)]
    list_supported: bool,

    #[arg(long, default_value_t = DEFAULT_MAX_ITEMS)]
    max_items: usize,

    #[arg(long, default_value_t = DEFAULT_MIN_LINES)]
    min_lines: usize,

    #[arg(long)]
    language: Option<String>,

    #[arg(long)]
    stdin: bool,

    #[arg(long)]
    lenient: bool,

    files: Vec<PathBuf>,
}

pub fn run() -> Result<()> {
    let args = Args::parse();
    run_with_config(args)
}

pub fn run_with_args(files: Vec<PathBuf>, max_depth: usize) -> Result<()> {
    run_with_config(Args {
        files,
        max_depth,
        format: OutputFormat::Markdown,
        header: false,
        list_supported: false,
        max_items: DEFAULT_MAX_ITEMS,
        min_lines: DEFAULT_MIN_LINES,
        language: None,
        stdin: false,
        lenient: false,
    })
}

fn run_with_config(args: Args) -> Result<()> {
    if args.list_supported {
        return list_supported(&args);
    }

    if args.stdin && !args.files.is_empty() {
        return cli_error("--stdin cannot be combined with file paths");
    }
    if args.stdin && args.language.is_none() {
        return cli_error("--stdin requires --language");
    }
    if !args.stdin && args.files.is_empty() {
        return Err(Error::NoInput);
    }

    let forced_language = parse_forced_language(args.language.as_deref())?;
    let mut files = Vec::new();
    let mut errors = Vec::new();

    if args.stdin {
        let mut source = String::new();
        io::stdin()
            .read_to_string(&mut source)
            .map_err(|source| Error::Io {
                path: PathBuf::from("<stdin>"),
                source,
            })?;
        match render_source(
            InputSource {
                path: PathBuf::from("<stdin>"),
                source,
                forced_language,
            },
            &args,
        ) {
            Ok(file) => files.push(file),
            Err(error) => errors.push(RenderError::from(error)),
        }
    } else {
        for path in &args.files {
            match read_file_source(path, forced_language) {
                Ok(input) => match render_source(input, &args) {
                    Ok(file) => files.push(file),
                    Err(error) => errors.push(RenderError::from(error)),
                },
                Err(error) => errors.push(RenderError::from(error)),
            }
        }
    }

    let had_errors = !errors.is_empty();
    match args.format {
        OutputFormat::Markdown => {
            print_markdown(&files, &errors, args.header);
        }
        OutputFormat::Json => {
            let response = JsonResponse { files, errors };
            println!(
                "{}",
                serde_json::to_string_pretty(&response).expect("serialize output")
            );
        }
    }

    if had_errors {
        Err(Error::Parse {
            path: PathBuf::from("wot"),
            message: "one or more files failed".into(),
        })
    } else {
        Ok(())
    }
}

#[derive(Debug)]
struct InputSource {
    path: PathBuf,
    source: String,
    forced_language: Option<Language>,
}

fn read_file_source(path: &Path, forced_language: Option<Language>) -> Result<InputSource> {
    if path.is_dir() {
        return Err(Error::Directory {
            path: path.to_path_buf(),
        });
    }

    let source = fs::read_to_string(path).map_err(|source| Error::Io {
        path: path.to_path_buf(),
        source,
    })?;
    Ok(InputSource {
        path: path.to_path_buf(),
        source,
        forced_language,
    })
}

fn render_source(input: InputSource, args: &Args) -> Result<RenderedFile> {
    let language = input
        .forced_language
        .or_else(|| Language::from_path(&input.path));
    let language = language.ok_or_else(|| Error::UnsupportedFile {
        path: input.path.clone(),
    })?;

    if line_count(&input.source) <= args.min_lines {
        return Ok(RenderedFile::verbatim(input.path, language, input.source));
    }

    let outline = parsers::parse_source(
        &input.path,
        &input.source,
        language,
        args.max_depth,
        args.lenient,
    )?;
    Ok(RenderedFile::outline(outline, args.max_items))
}

fn line_count(source: &str) -> usize {
    source.lines().count()
}

fn parse_forced_language(language: Option<&str>) -> Result<Option<Language>> {
    language
        .map(|name| {
            Language::from_name(name).ok_or_else(|| Error::Parse {
                path: PathBuf::from("wot"),
                message: format!("unsupported language `{name}`"),
            })
        })
        .transpose()
}

fn cli_error(message: impl Into<String>) -> Result<()> {
    Err(Error::Parse {
        path: PathBuf::from("wot"),
        message: message.into(),
    })
}

#[derive(Debug, Clone, Serialize)]
struct JsonResponse {
    files: Vec<RenderedFile>,
    errors: Vec<RenderError>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "mode", rename_all = "lowercase")]
enum RenderedFile {
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
    fn outline(outline: Outline, max_items: usize) -> Self {
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

    fn verbatim(path: PathBuf, language: Language, content: String) -> Self {
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
struct JsonNode {
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
struct RenderError {
    message: String,
}

impl From<Error> for RenderError {
    fn from(error: Error) -> Self {
        Self {
            message: error.to_string(),
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

fn print_markdown(files: &[RenderedFile], errors: &[RenderError], header: bool) {
    let mut first_output = true;
    for file in files {
        if !first_output {
            println!();
        }
        match file {
            RenderedFile::Outline { nodes, .. } => {
                if header {
                    println!("# {}", file.path());
                }
                print_markdown_nodes(nodes, 0);
            }
            RenderedFile::Verbatim { content, .. } => {
                if header {
                    println!("# {}", file.path());
                }
                print!("{content}");
            }
        }
        first_output = false;
    }

    for error in errors {
        eprintln!("{}", error.message);
    }
}

fn print_markdown_nodes(nodes: &[JsonNode], depth: usize) {
    let indent = "  ".repeat(depth);
    for node in nodes {
        println!("{}- {} [{}]", indent, node.label, node.range.display);
        print_markdown_nodes(&node.children, depth + 1);
    }
}

fn list_supported(args: &Args) -> Result<()> {
    if args.stdin || !args.files.is_empty() {
        return cli_error("--list-supported cannot be combined with input files or --stdin");
    }

    match args.format {
        OutputFormat::Markdown => {
            for spec in Language::supported_specs() {
                println!(
                    "- {}: extensions [{}], filenames [{}], backend {}",
                    spec.language,
                    spec.extensions.join(", "),
                    spec.filenames.join(", "),
                    spec.backend
                );
            }
        }
        OutputFormat::Json => {
            let response = SupportedResponse {
                languages: Language::supported_specs()
                    .iter()
                    .map(SupportedLanguage::from)
                    .collect(),
            };
            println!(
                "{}",
                serde_json::to_string_pretty(&response).expect("serialize supported languages")
            );
        }
    }
    Ok(())
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
