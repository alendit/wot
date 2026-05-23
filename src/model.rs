use serde::Serialize;
use std::fmt;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
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
    Org,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LanguageSpec {
    pub language: Language,
    pub names: &'static [&'static str],
    pub extensions: &'static [&'static str],
    pub filenames: &'static [&'static str],
    pub backend: &'static str,
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
            Some("org") => Some(Self::Org),
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

    pub fn from_name(name: &str) -> Option<Self> {
        let normalized = name.trim().to_ascii_lowercase();
        Self::supported_specs()
            .iter()
            .find(|spec| spec.names.iter().any(|alias| *alias == normalized))
            .map(|spec| spec.language)
    }

    pub fn supported_specs() -> &'static [LanguageSpec] {
        &[
            LanguageSpec {
                language: Self::Rust,
                names: &["rust", "rs"],
                extensions: &[".rs"],
                filenames: &[],
                backend: "tree-sitter",
            },
            LanguageSpec {
                language: Self::TypeScript,
                names: &["typescript", "ts", "tsx"],
                extensions: &[".ts", ".tsx", ".mts", ".cts"],
                filenames: &[],
                backend: "tree-sitter",
            },
            LanguageSpec {
                language: Self::JavaScript,
                names: &["javascript", "js", "jsx"],
                extensions: &[".js", ".jsx", ".mjs", ".cjs"],
                filenames: &[],
                backend: "tree-sitter",
            },
            LanguageSpec {
                language: Self::Go,
                names: &["go"],
                extensions: &[".go"],
                filenames: &[],
                backend: "tree-sitter",
            },
            LanguageSpec {
                language: Self::C,
                names: &["c"],
                extensions: &[".c", ".h"],
                filenames: &[],
                backend: "tree-sitter",
            },
            LanguageSpec {
                language: Self::Cpp,
                names: &["cpp", "c++"],
                extensions: &[".cc", ".cpp", ".cxx", ".hpp", ".hh", ".hxx"],
                filenames: &[],
                backend: "tree-sitter",
            },
            LanguageSpec {
                language: Self::Java,
                names: &["java"],
                extensions: &[".java"],
                filenames: &[],
                backend: "tree-sitter",
            },
            LanguageSpec {
                language: Self::Kotlin,
                names: &["kotlin", "kt"],
                extensions: &[".kt", ".kts"],
                filenames: &[],
                backend: "tree-sitter",
            },
            LanguageSpec {
                language: Self::CSharp,
                names: &["csharp", "c#"],
                extensions: &[".cs"],
                filenames: &[],
                backend: "tree-sitter",
            },
            LanguageSpec {
                language: Self::Shell,
                names: &["shell", "sh", "bash", "zsh"],
                extensions: &[".sh", ".bash", ".zsh"],
                filenames: &[],
                backend: "tree-sitter",
            },
            LanguageSpec {
                language: Self::Clojure,
                names: &["clojure", "clj", "cljs", "cljc", "bb"],
                extensions: &[".clj", ".cljs", ".cljc", ".bb"],
                filenames: &[],
                backend: "tree-sitter",
            },
            LanguageSpec {
                language: Self::Elisp,
                names: &["elisp", "emacs-lisp"],
                extensions: &[".el"],
                filenames: &[],
                backend: "tree-sitter",
            },
            LanguageSpec {
                language: Self::Markdown,
                names: &["markdown", "md"],
                extensions: &[".md", ".markdown"],
                filenames: &[],
                backend: "pulldown-cmark",
            },
            LanguageSpec {
                language: Self::Org,
                names: &["org", "org-mode"],
                extensions: &[".org"],
                filenames: &[],
                backend: "scanner",
            },
            LanguageSpec {
                language: Self::Python,
                names: &["python", "py"],
                extensions: &[".py"],
                filenames: &[],
                backend: "ruff-python-parser",
            },
            LanguageSpec {
                language: Self::Json,
                names: &["json"],
                extensions: &[".json"],
                filenames: &[],
                backend: "serde_json + scanner",
            },
            LanguageSpec {
                language: Self::Yaml,
                names: &["yaml", "yml"],
                extensions: &[".yaml", ".yml"],
                filenames: &[],
                backend: "serde_yaml + scanner",
            },
            LanguageSpec {
                language: Self::Toml,
                names: &["toml"],
                extensions: &[".toml"],
                filenames: &[],
                backend: "toml + scanner",
            },
            LanguageSpec {
                language: Self::Ini,
                names: &["ini"],
                extensions: &[".ini"],
                filenames: &[],
                backend: "scanner",
            },
            LanguageSpec {
                language: Self::Dotenv,
                names: &["dotenv", "env"],
                extensions: &[],
                filenames: &[".env", ".env.*"],
                backend: "scanner",
            },
            LanguageSpec {
                language: Self::Xml,
                names: &["xml", "svg", "plist"],
                extensions: &[".xml", ".svg", ".plist"],
                filenames: &[],
                backend: "scanner",
            },
            LanguageSpec {
                language: Self::Hcl,
                names: &["hcl", "terraform", "tf", "tfvars"],
                extensions: &[".hcl", ".tf", ".tfvars"],
                filenames: &[],
                backend: "scanner",
            },
            LanguageSpec {
                language: Self::Dockerfile,
                names: &["dockerfile", "containerfile"],
                extensions: &[".dockerfile"],
                filenames: &["Dockerfile", "Containerfile"],
                backend: "scanner",
            },
            LanguageSpec {
                language: Self::Notebook,
                names: &["notebook", "ipynb", "jupyter"],
                extensions: &[".ipynb"],
                filenames: &[],
                backend: "serde_json + embedded parsers",
            },
        ]
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
            Self::Org => write!(formatter, "org"),
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Position {
    pub line: usize,
    pub column: usize,
}
