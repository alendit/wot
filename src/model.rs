use std::fmt;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    Markdown,
    Python,
    Json,
    Yaml,
    Toml,
    Ini,
    Dotenv,
    Xml,
    Hcl,
    Dockerfile,
    Notebook,
    Rust,
    TypeScript,
    JavaScript,
    Go,
    C,
    Cpp,
    Java,
    Kotlin,
    CSharp,
    Shell,
    Clojure,
    Elisp,
}

impl Language {
    pub fn from_path(path: &std::path::Path) -> Option<Self> {
        let file_name = path.file_name().and_then(|name| name.to_str())?;
        if file_name == "Dockerfile"
            || file_name == "Containerfile"
            || file_name.ends_with(".dockerfile")
        {
            return Some(Self::Dockerfile);
        }
        if file_name == ".env" || file_name.starts_with(".env.") {
            return Some(Self::Dotenv);
        }

        match path.extension().and_then(|extension| extension.to_str()) {
            Some("rs") => Some(Self::Rust),
            Some("ts" | "tsx" | "mts" | "cts") => Some(Self::TypeScript),
            Some("js" | "jsx" | "mjs" | "cjs") => Some(Self::JavaScript),
            Some("go") => Some(Self::Go),
            Some("c" | "h") => Some(Self::C),
            Some("cc" | "cpp" | "cxx" | "hpp" | "hh" | "hxx") => Some(Self::Cpp),
            Some("java") => Some(Self::Java),
            Some("kt" | "kts") => Some(Self::Kotlin),
            Some("cs") => Some(Self::CSharp),
            Some("sh" | "bash" | "zsh") => Some(Self::Shell),
            Some("clj" | "cljs" | "cljc" | "bb") => Some(Self::Clojure),
            Some("el") => Some(Self::Elisp),
            Some("md" | "markdown") => Some(Self::Markdown),
            Some("py") => Some(Self::Python),
            Some("json") => Some(Self::Json),
            Some("yaml" | "yml") => Some(Self::Yaml),
            Some("toml") => Some(Self::Toml),
            Some("ini") => Some(Self::Ini),
            Some("xml" | "svg" | "plist") => Some(Self::Xml),
            Some("hcl" | "tf" | "tfvars") => Some(Self::Hcl),
            Some("ipynb") => Some(Self::Notebook),
            _ => None,
        }
    }
}

impl fmt::Display for Language {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Markdown => write!(formatter, "markdown"),
            Self::Python => write!(formatter, "python"),
            Self::Json => write!(formatter, "json"),
            Self::Yaml => write!(formatter, "yaml"),
            Self::Toml => write!(formatter, "toml"),
            Self::Ini => write!(formatter, "ini"),
            Self::Dotenv => write!(formatter, "dotenv"),
            Self::Xml => write!(formatter, "xml"),
            Self::Hcl => write!(formatter, "hcl"),
            Self::Dockerfile => write!(formatter, "dockerfile"),
            Self::Notebook => write!(formatter, "notebook"),
            Self::Rust => write!(formatter, "rust"),
            Self::TypeScript => write!(formatter, "typescript"),
            Self::JavaScript => write!(formatter, "javascript"),
            Self::Go => write!(formatter, "go"),
            Self::C => write!(formatter, "c"),
            Self::Cpp => write!(formatter, "cpp"),
            Self::Java => write!(formatter, "java"),
            Self::Kotlin => write!(formatter, "kotlin"),
            Self::CSharp => write!(formatter, "csharp"),
            Self::Shell => write!(formatter, "shell"),
            Self::Clojure => write!(formatter, "clojure"),
            Self::Elisp => write!(formatter, "elisp"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outline {
    pub path: PathBuf,
    pub language: Language,
    pub nodes: Vec<OutlineNode>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutlineNode {
    pub label: String,
    pub kind: NodeKind,
    pub range: SourceRange,
    pub children: Vec<OutlineNode>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeKind {
    Heading,
    Class,
    Function,
    JsonProperty,
    JsonArrayElement,
    ConfigKey,
    ConfigSection,
    ConfigArrayElement,
    XmlElement,
    HclBlock,
    DockerStage,
    DockerInstruction,
    NotebookCell,
    Import,
    Export,
    Type,
    Method,
    Component,
    Module,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceRange {
    pub start: Position,
    pub end: Position,
    pub precise: bool,
}

impl SourceRange {
    pub const fn lines(start_line: usize, end_line: usize) -> Self {
        Self {
            start: Position {
                line: start_line,
                column: 1,
            },
            end: Position {
                line: end_line,
                column: 1,
            },
            precise: false,
        }
    }

    pub const fn spans(
        start_line: usize,
        start_column: usize,
        end_line: usize,
        end_column: usize,
    ) -> Self {
        Self {
            start: Position {
                line: start_line,
                column: start_column,
            },
            end: Position {
                line: end_line,
                column: end_column,
            },
            precise: true,
        }
    }

    pub fn display(self) -> String {
        if self.precise {
            format!(
                "L{}:C{}-L{}:C{}",
                self.start.line, self.start.column, self.end.line, self.end.column
            )
        } else if self.start.line == self.end.line {
            format!("L{}", self.start.line)
        } else {
            format!("L{}-L{}", self.start.line, self.end.line)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    pub line: usize,
    pub column: usize,
}
