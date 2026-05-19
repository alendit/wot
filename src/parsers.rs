use std::path::Path;

use crate::error::{Error, Result};
use crate::model::{Language, Outline};

pub mod json;
pub mod markdown;
pub mod python;

pub fn parse_file(path: &Path, source: &str, max_depth: usize) -> Result<Outline> {
    let language = Language::from_path(path).ok_or_else(|| Error::UnsupportedFile {
        path: path.to_path_buf(),
    })?;

    match language {
        Language::Markdown => markdown::parse(path, source, max_depth),
        Language::Python => python::parse(path, source, max_depth),
        Language::Json => json::parse(path, source, max_depth),
    }
}
