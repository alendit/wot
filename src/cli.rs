use std::env;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use clap::{Args as ClapArgs, Parser, Subcommand, ValueEnum};
use serde_json::{json, Value};

use crate::error::{Error, Result};
use crate::model::Language;
use crate::parsers;
use crate::renderer::{self, RenderError, RenderedFile};

const DEFAULT_MAX_DEPTH: usize = 3;
const DEFAULT_MAX_ITEMS: usize = 200;
const DEFAULT_MIN_LINES: usize = 40;
const SKILL_NAME: &str = "create-file-outline";
const SKILL_CONTENT: &str = include_str!("../skills/create-file-outline/SKILL.md");
const HOOK_COMMAND: &str = "wot hook-check";
const HOOK_CONTEXT: &str = "wot: This looks like broad file exploration. If you are deciding which parts of files are worth reading, reduce selection uncertainty first: use `rg --files` for candidates, then `wot --min-lines 0 <file>...` for outlines and line ranges before broad reads.";

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum OutputFormat {
    Markdown,
    Json,
}

#[derive(Debug, Parser)]
#[command(
    name = "wot",
    about = "Create compact outlines from source, config, and document files",
    override_usage = "wot [OPTIONS] <file>...\n       wot setup [OPTIONS]\n       wot hook-check",
    long_about = "Create compact Markdown table-of-contents style outlines from source, config, and document files.\n\nSupported inputs include Rust, TypeScript/JavaScript, Go, C/C++, Java, Kotlin, C#, shell, Clojure, Emacs Lisp, Markdown, Python, JSON, YAML, TOML, INI, .env, XML/SVG/plist, HCL/Terraform, Dockerfile/Containerfile, and Jupyter notebooks.\n\nRanges are 1-based inclusive line ranges. When line-only ranges would be ambiguous, wot prints 1-based start-inclusive/end-exclusive columns as Lx:Cy-Lx:Cz."
)]
struct Args {
    #[command(subcommand)]
    command: Option<Command>,

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

#[derive(Debug, Subcommand)]
enum Command {
    #[command(about = "Install the bundled agent skill")]
    Setup(SetupArgs),

    #[command(about = "Run the advisory PreToolUse hook check")]
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

    #[arg(help = "Also install advisory PreToolUse hooks", long)]
    hooks: bool,
}

pub fn run() -> Result<()> {
    let args = Args::parse();
    run_with_config(args)
}

pub fn run_with_args(files: Vec<PathBuf>, max_depth: usize) -> Result<()> {
    run_with_config(Args {
        command: None,
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
    if let Some(command) = &args.command {
        return match command {
            Command::Setup(setup_args) => setup(setup_args),
            Command::HookCheck => hook_check(),
        };
    }

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
            print!("{}", renderer::render_markdown(&files, args.header));
            for error in &errors {
                eprintln!("{}", error.message);
            }
        }
        OutputFormat::Json => {
            println!("{}", renderer::render_json(files, errors));
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
    let entry = hook_entry("Bash|Read|Glob|Grep");
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

    if should_emit_hook_context(&value) {
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
        Some("Bash") => value
            .get("tool_input")
            .and_then(|tool_input| tool_input.get("command"))
            .and_then(Value::as_str)
            .is_some_and(is_broad_shell_exploration),
        Some("Read") => is_full_file_read(value.get("tool_input")),
        Some("Glob") => true,
        Some("Grep") => false,
        _ => false,
    }
}

fn is_full_file_read(tool_input: Option<&Value>) -> bool {
    let Some(tool_input) = tool_input else {
        return false;
    };
    tool_input.get("file_path").is_some()
        && tool_input.get("offset").is_none()
        && tool_input.get("limit").is_none()
}

fn is_broad_shell_exploration(command: &str) -> bool {
    let command = command.trim();
    if command.is_empty()
        || command.starts_with("wot ")
        || command == "wot"
        || command.starts_with("git ")
        || command.starts_with("cargo ")
        || command.starts_with("rtk cargo ")
    {
        return false;
    }

    let tokens = shell_words(command);
    if tokens.is_empty() {
        return false;
    }

    match tokens[0].as_str() {
        "cat" | "head" | "tail" | "nl" | "find" | "fd" => true,
        "ls" => tokens.iter().any(|token| token.contains('R')),
        "sed" => is_broad_sed(&tokens),
        "rg" | "grep" | "ripgrep" => is_broad_search(&tokens),
        _ => false,
    }
}

fn is_broad_sed(tokens: &[String]) -> bool {
    if !tokens
        .iter()
        .any(|token| token == "-n" || token.contains('n'))
    {
        return true;
    }

    tokens
        .iter()
        .find_map(|token| parse_sed_line_span(token))
        .is_some_and(|line_count| line_count > 80)
}

fn parse_sed_line_span(token: &str) -> Option<usize> {
    let token = token.strip_suffix('p').unwrap_or(token);
    let (start, end) = token.split_once(',')?;
    let start = start.parse::<usize>().ok()?;
    let end = end.parse::<usize>().ok()?;
    end.checked_sub(start).map(|delta| delta + 1)
}

fn is_broad_search(tokens: &[String]) -> bool {
    if tokens
        .iter()
        .skip(1)
        .any(|token| token == "--files" || token == "-l")
    {
        return false;
    }

    tokens
        .iter()
        .skip(1)
        .all(|token| token.starts_with('-') || token == ".")
}

fn shell_words(command: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut current = String::new();
    let mut quote = None;

    for ch in command.chars() {
        match (quote, ch) {
            (Some(q), c) if c == q => quote = None,
            (Some(_), c) => current.push(c),
            (None, '\'' | '"') => quote = Some(ch),
            (None, c) if c.is_whitespace() => {
                if !current.is_empty() {
                    words.push(std::mem::take(&mut current));
                }
            }
            (None, c) => current.push(c),
        }
    }

    if !current.is_empty() {
        words.push(current);
    }
    words
}

fn list_supported(args: &Args) -> Result<()> {
    if args.stdin || !args.files.is_empty() {
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
