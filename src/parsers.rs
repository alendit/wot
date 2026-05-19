use std::path::Path;

use crate::error::{Error, Result};
use crate::model::{Language, Outline};

mod structured;

mod tree_sitter_code;

pub mod dockerfile;
pub mod dotenv;
pub mod hcl;
pub mod ini;
pub mod json;
pub mod markdown;
pub mod notebook;
pub mod python;
pub mod toml;
pub mod xml;
pub mod yaml;

pub fn parse_file(path: &Path, source: &str, max_depth: usize) -> Result<Outline> {
    let language = Language::from_path(path).ok_or_else(|| Error::UnsupportedFile {
        path: path.to_path_buf(),
    })?;
    parse_source(path, source, language, max_depth, false)
}

pub fn parse_source(
    path: &Path,
    source: &str,
    language: Language,
    max_depth: usize,
    lenient: bool,
) -> Result<Outline> {
    match language {
        Language::Markdown => markdown::parse(path, source, max_depth),
        Language::Python => python::parse(path, source, max_depth),
        Language::Json => json::parse(path, source, max_depth),
        Language::Yaml => yaml::parse_with_options(path, source, max_depth, lenient),
        Language::Toml => toml::parse_with_options(path, source, max_depth, lenient),
        Language::Ini => ini::parse(path, source, max_depth),
        Language::Dotenv => dotenv::parse(path, source, max_depth),
        Language::Xml => xml::parse_with_options(path, source, max_depth, lenient),
        Language::Hcl => hcl::parse_with_options(path, source, max_depth, lenient),
        Language::Dockerfile => dockerfile::parse(path, source, max_depth),
        Language::Notebook => notebook::parse(path, source, max_depth),
        Language::Rust
        | Language::TypeScript
        | Language::JavaScript
        | Language::Go
        | Language::C
        | Language::Cpp
        | Language::Java
        | Language::Kotlin
        | Language::CSharp
        | Language::Shell
        | Language::Clojure
        | Language::Elisp => tree_sitter_code::parse(path, source, language, max_depth),
    }
}
