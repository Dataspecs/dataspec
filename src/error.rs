use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("missing H1 heading in {path}")]
    MissingTitle { path: PathBuf },

    #[error("missing ## Type section in {path}")]
    MissingType { path: PathBuf },

    #[error("unknown entity type `{entity_type}` in {path}")]
    UnknownEntityType { path: PathBuf, entity_type: String },

    #[error("failed to read {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

pub type Result<T> = std::result::Result<T, ParseError>;
