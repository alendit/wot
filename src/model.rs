use std::fmt;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    Markdown,
    Python,
    Json,
}

impl Language {
    pub fn from_path(path: &std::path::Path) -> Option<Self> {
        match path.extension().and_then(|extension| extension.to_str()) {
            Some("md" | "markdown") => Some(Self::Markdown),
            Some("py") => Some(Self::Python),
            Some("json") => Some(Self::Json),
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
