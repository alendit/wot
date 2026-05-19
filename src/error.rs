use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("{path}: is a directory")]
    Directory { path: PathBuf },

    #[error("{path}: unsupported file type")]
    UnsupportedFile { path: PathBuf },

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
