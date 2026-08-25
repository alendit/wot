use std::env;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use clap::{Args as ClapArgs, Parser, Subcommand, ValueEnum};
use serde_json::{json, Value};

use crate::discovery::{self, DiscoveredDirectory, DiscoveredEntry};
use crate::error::{Error, Result};
use crate::hook;
use crate::model::Language;
use crate::parsers;
use crate::renderer::{
    self, RenderError, RenderedDirectory, RenderedDirectoryEntry, RenderedFile, RenderedRoot,
};

const DEFAULT_MAX_DEPTH: usize = 3;
const DEFAULT_WALK_DEPTH: usize = 3;
const DEFAULT_MAX_ITEMS: usize = 200;
const DEFAULT_MIN_LINES: usize = 40;
const SKILL_NAME: &str = "create-file-outline";
const SKILL_CONTENT: &str = include_str!("../skills/create-file-outline/SKILL.md");
const HOOK_COMMAND: &str = "wot hook-check";
const HOOK_CONTEXT: &str = "Use wot for a file overview.";

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum OutputFormat {
    Markdown,
    Json,
}

#[derive(Debug, Parser)]
#[command(
    name = "wot",
    about = "Create compact outlines from source, config, and document files",
    override_usage = "wot [OPTIONS] <path>...\n       wot setup [OPTIONS]\n       wot hook-check",
    long_about = "Create compact Markdown table-of-contents style outlines from source, config, and document files. Directory inputs are traversed recursively.\n\nSupported inputs include Rust, TypeScript/JavaScript, Go, C/C++, Java, Kotlin, C#, shell, Clojure, Emacs Lisp, Markdown, Python, JSON, YAML, TOML, INI, .env, XML/SVG/plist, HCL/Terraform, Dockerfile/Containerfile, and Jupyter notebooks.\n\nRanges are 1-based inclusive line ranges. When line-only ranges would be ambiguous, wot prints 1-based start-inclusive/end-exclusive columns as Lx:Cy-Lx:Cz."
)]
struct Args {
    #[command(subcommand)]
    command: Option<Command>,

    #[arg(long, default_value_t = DEFAULT_MAX_DEPTH)]
    max_depth: usize,

    #[arg(
        long,
        default_value_t = DEFAULT_WALK_DEPTH,
        help = "Maximum filesystem depth for directory inputs (target directory is depth 0)"
    )]
    walk_depth: usize,

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

    paths: Vec<PathBuf>,
}

#[derive(Debug, Subcommand)]
enum Command {
    #[command(about = "Install the bundled agent skill")]
    Setup(SetupArgs),

    #[command(about = "Run the PreToolUse hook command-rewrite policy")]
    HookCheck,
}

#[derive(Debug, ClapArgs)]
struct SetupArgs {
    #[arg(
        short = 'g',
        long,
        help = "Install to the user home skill roots instead of the current project"
    )]
    global: bool,

    #[arg(help = "Also install to .claude/skills or ~/.claude/skills", long)]
    claude: bool,

    #[arg(help = "Also install PreToolUse command-rewrite hooks", long)]
    hooks: bool,
}

pub fn run() -> Result<()> {
    let args = Args::parse();
    run_with_config(args)
}

pub fn run_with_args(paths: Vec<PathBuf>, max_depth: usize) -> Result<()> {
    run_with_config(Args {
        command: None,
        paths,
        max_depth,
        walk_depth: DEFAULT_WALK_DEPTH,
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
    if let Some(command) = &args.command {
        return match command {
            Command::Setup(setup_args) => setup(setup_args),
            Command::HookCheck => hook_check(),
        };
    }

    if args.list_supported {
        return list_supported(&args);
    }

    if args.stdin && !args.paths.is_empty() {
        return cli_error("--stdin cannot be combined with file paths");
    }
    if args.stdin && args.language.is_none() {
        return cli_error("--stdin requires --language");
    }
    if !args.stdin && args.paths.is_empty() {
        return Err(Error::NoInput);
    }

    if args.language.is_some() && args.paths.iter().any(|path| path.is_dir()) {
        return cli_error("--language cannot be combined with directory inputs");
    }

    let forced_language = parse_forced_language(args.language.as_deref())?;
    let mut roots = Vec::new();
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
            false,
        ) {
            Ok(file) => roots.push(RenderedRoot::File(file)),
            Err(error) => errors.push(RenderError::from(error)),
        }
    } else {
        for path in &args.paths {
            if path.is_dir() {
                let discovered = discovery::discover(path, args.walk_depth);
                errors.extend(discovered.errors.into_iter().map(RenderError::from));
                roots.push(RenderedRoot::Directory(render_discovered_directory(
                    discovered.root,
                    &args,
                    &mut errors,
                )));
            } else {
                match read_file_source(path, forced_language) {
                    Ok(input) => match render_source(input, &args, false) {
                        Ok(file) => roots.push(RenderedRoot::File(file)),
                        Err(error) => errors.push(RenderError::from(error)),
                    },
                    Err(error) => errors.push(RenderError::from(error)),
                }
            }
        }
    }

    let had_errors = !errors.is_empty();
    match args.format {
        OutputFormat::Markdown => {
            print!("{}", renderer::render_roots_markdown(&roots, args.header));
            for error in &errors {
                eprintln!("{}", error.message);
            }
        }
        OutputFormat::Json => {
            println!("{}", renderer::render_roots_json(&roots, errors));
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

fn render_source(input: InputSource, args: &Args, force_outline: bool) -> Result<RenderedFile> {
    let language = input
        .forced_language
        .or_else(|| Language::from_path(&input.path));
    let language = language.ok_or_else(|| Error::UnsupportedFile {
        path: input.path.clone(),
    })?;

    if !force_outline && line_count(&input.source) <= args.min_lines {
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

fn render_discovered_directory(
    directory: DiscoveredDirectory,
    args: &Args,
    errors: &mut Vec<RenderError>,
) -> RenderedDirectory {
    let mut entries = Vec::new();

    for entry in directory.entries {
        match entry {
            DiscoveredEntry::Directory(child) => {
                let child = render_discovered_directory(child, args, errors);
                if child.depth_limited || !child.entries.is_empty() {
                    entries.push(RenderedDirectoryEntry::Directory(child));
                }
            }
            DiscoveredEntry::File(path) => {
                let force_outline = Language::from_path(&path) == Some(Language::Dotenv);
                match read_file_source(&path, None)
                    .and_then(|input| render_source(input, args, force_outline))
                {
                    Ok(file) => entries.push(RenderedDirectoryEntry::File(file)),
                    Err(error) => errors.push(RenderError::from(error)),
                }
            }
        }
    }

    RenderedDirectory {
        path: directory.path.display().to_string(),
        walk_depth: directory.walk_depth,
        depth_limited: directory.depth_limited,
        entries,
    }
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

fn setup(args: &SetupArgs) -> Result<()> {
    let destinations = setup_destinations(args)?;
    for destination in &destinations {
        install_skill(destination)?;
        println!("installed {SKILL_NAME} skill to {}", destination.display());
    }
    if args.hooks {
        let base = setup_base(args)?;
        install_codex_hook(&base)?;
        if args.claude {
            install_claude_hook(&base)?;
        }
    }
    Ok(())
}

fn setup_destinations(args: &SetupArgs) -> Result<Vec<PathBuf>> {
    let base = setup_base(args)?;

    let mut destinations = vec![skill_destination(&base, ".agents")];
    if args.claude {
        destinations.push(skill_destination(&base, ".claude"));
    }
    Ok(destinations)
}

fn setup_base(args: &SetupArgs) -> Result<PathBuf> {
    if args.global {
        home_dir()
    } else {
        env::current_dir().map_err(|source| Error::Io {
            path: PathBuf::from("."),
            source,
        })
    }
}

fn home_dir() -> Result<PathBuf> {
    env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| Error::Parse {
            path: PathBuf::from("wot"),
            message: "HOME is not set".into(),
        })
}

fn skill_destination(base: &Path, root_name: &str) -> PathBuf {
    base.join(root_name)
        .join("skills")
        .join(SKILL_NAME)
        .join("SKILL.md")
}

fn install_skill(destination: &Path) -> Result<()> {
    fs::create_dir_all(
        destination
            .parent()
            .expect("skill destination has a parent"),
    )
    .map_err(|source| Error::Io {
        path: destination.to_path_buf(),
        source,
    })?;
    fs::write(destination, SKILL_CONTENT).map_err(|source| Error::Io {
        path: destination.to_path_buf(),
        source,
    })
}

fn install_codex_hook(base: &Path) -> Result<()> {
    let path = base.join(".codex").join("hooks.json");
    let entry = hook_entry("Bash");
    install_pre_tool_hook(&path, entry)?;
    println!("installed wot hook to {}", path.display());
    Ok(())
}

fn install_claude_hook(base: &Path) -> Result<()> {
    let path = base.join(".claude").join("settings.json");
    let entry = hook_entry("Bash|Read");
    install_pre_tool_hook(&path, entry)?;
    println!("installed wot hook to {}", path.display());
    Ok(())
}

fn hook_entry(matcher: &str) -> Value {
    json!({
        "matcher": matcher,
        "hooks": [
            {
                "type": "command",
                "command": HOOK_COMMAND
            }
        ]
    })
}

fn install_pre_tool_hook(path: &Path, entry: Value) -> Result<()> {
    let mut root = read_json_object_or_empty(path)?;
    let pre_tool = pre_tool_hooks_array_mut(&mut root, path)?;
    pre_tool.retain(|hook| !mentions_wot_hook_check(hook));
    pre_tool.push(entry);
    write_json(path, &root)
}

fn read_json_object_or_empty(path: &Path) -> Result<Value> {
    if !path.exists() {
        return Ok(json!({}));
    }

    let content = fs::read_to_string(path).map_err(|source| Error::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let value: Value = serde_json::from_str(&content).map_err(|source| Error::Parse {
        path: path.to_path_buf(),
        message: format!("invalid JSON: {source}"),
    })?;
    if value.is_object() {
        Ok(value)
    } else {
        Err(Error::Parse {
            path: path.to_path_buf(),
            message: "expected JSON object".into(),
        })
    }
}

fn pre_tool_hooks_array_mut<'a>(root: &'a mut Value, path: &Path) -> Result<&'a mut Vec<Value>> {
    let root_object = root.as_object_mut().ok_or_else(|| Error::Parse {
        path: path.to_path_buf(),
        message: "expected JSON object".into(),
    })?;
    let hooks = root_object.entry("hooks").or_insert_with(|| json!({}));
    let hooks_object = hooks.as_object_mut().ok_or_else(|| Error::Parse {
        path: path.to_path_buf(),
        message: "expected hooks object".into(),
    })?;
    let pre_tool = hooks_object
        .entry("PreToolUse")
        .or_insert_with(|| json!([]));
    pre_tool.as_array_mut().ok_or_else(|| Error::Parse {
        path: path.to_path_buf(),
        message: "expected hooks.PreToolUse array".into(),
    })
}

fn mentions_wot_hook_check(value: &Value) -> bool {
    serde_json::to_string(value)
        .map(|text| text.contains(HOOK_COMMAND))
        .unwrap_or(false)
}

fn write_json(path: &Path, value: &Value) -> Result<()> {
    fs::create_dir_all(path.parent().expect("hook path has a parent")).map_err(|source| {
        Error::Io {
            path: path.to_path_buf(),
            source,
        }
    })?;
    let content = serde_json::to_string_pretty(value).map_err(|source| Error::Parse {
        path: path.to_path_buf(),
        message: source.to_string(),
    })?;
    fs::write(path, format!("{content}\n")).map_err(|source| Error::Io {
        path: path.to_path_buf(),
        source,
    })
}

fn hook_check() -> Result<()> {
    let mut input = String::new();
    io::stdin()
        .read_to_string(&mut input)
        .map_err(|source| Error::Io {
            path: PathBuf::from("<stdin>"),
            source,
        })?;

    let Ok(value) = serde_json::from_str::<Value>(&input) else {
        return Ok(());
    };

    if let Some(command) = bash_command(&value).and_then(hook::rewrite_bash_command) {
        println!(
            "{}",
            json!({
                "hookSpecificOutput": {
                    "hookEventName": "PreToolUse",
                    "permissionDecision": "allow",
                    "updatedInput": {
                        "command": command
                    }
                }
            })
        );
    } else if should_emit_hook_context(&value) {
        println!(
            "{}",
            json!({
                "hookSpecificOutput": {
                    "hookEventName": "PreToolUse",
                    "additionalContext": HOOK_CONTEXT
                }
            })
        );
    }
    Ok(())
}

fn should_emit_hook_context(value: &Value) -> bool {
    match value.get("tool_name").and_then(Value::as_str) {
        Some("Read") => is_full_file_read(value.get("tool_input")),
        _ => false,
    }
}

fn bash_command(value: &Value) -> Option<&str> {
    if value.get("tool_name").and_then(Value::as_str) != Some("Bash") {
        return None;
    }
    value.get("tool_input")?.get("command")?.as_str()
}

fn is_full_file_read(tool_input: Option<&Value>) -> bool {
    let Some(tool_input) = tool_input else {
        return false;
    };
    tool_input.get("file_path").is_some()
        && tool_input.get("offset").is_none()
        && tool_input.get("limit").is_none()
}

fn list_supported(args: &Args) -> Result<()> {
    if args.stdin || !args.paths.is_empty() {
        return cli_error("--list-supported cannot be combined with input files or --stdin");
    }

    match args.format {
        OutputFormat::Markdown => {
            print!(
                "{}",
                renderer::render_supported_markdown(Language::supported_specs())
            );
        }
        OutputFormat::Json => {
            println!(
                "{}",
                renderer::render_supported_json(Language::supported_specs())
            );
        }
    }
    Ok(())
}
