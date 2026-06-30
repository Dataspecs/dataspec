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

    #[error("duplicate {kind} `{name}`")]
    DuplicateEntity { kind: String, name: String },

    #[error("multiple config entities found")]
    MultipleConfig,

    #[error("invalid entity name for static generation: `{name}`")]
    InvalidEntityName { name: String },

    #[error("template `{template}` not found when compiling `{entity}`")]
    TemplateNotFound { template: String, entity: String },

    #[error("prop `props__{prop}` not found when compiling `{entity}`")]
    PropNotFound { prop: String, entity: String },
}

pub type Result<T> = std::result::Result<T, ParseError>;
