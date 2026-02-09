use thiserror::Error;

/// Errors that can occur during storage / import operations.
#[derive(Debug, Error)]
pub enum StorageError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("database error: {0}")]
    Db(#[from] rusqlite::Error),

    #[error("finding folder is missing a markdown (.md) file: {0}")]
    MissingMarkdown(String),

    #[error("failed to parse finding markdown: {0}")]
    ParseError(String),

    #[error("POGDIR is not set and could not be determined")]
    NoPogDir,
}

/// Convenience alias used throughout the crate.
pub type Result<T> = std::result::Result<T, StorageError>;
