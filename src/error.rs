use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("{path}: unsupported file type")]
    UnsupportedFile { path: PathBuf },

    #[error("{path}: directory traversal failed: {message}")]
    DirectoryTraversal { path: PathBuf, message: String },

    #[error("{path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("{path}: {message}")]
    Parse { path: PathBuf, message: String },

    #[error("no input files provided")]
    NoInput,
}

pub type Result<T> = std::result::Result<T, Error>;
