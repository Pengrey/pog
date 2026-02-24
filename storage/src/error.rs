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

    #[error("template rendering error: {0}")]
    TemplateError(String),

    #[error("PDF compilation error: {0}")]
    PdfError(String),

    #[error("POGDIR is not set and could not be determined")]
    NoPogDir,

    #[error("no client selected – use --client <name> or set a default with `pog client default <name>`")]
    NoClientSelected,

    #[error("client not found: {0}")]
    ClientNotFound(String),
}

/// Convenience alias used throughout the crate.
pub type Result<T> = std::result::Result<T, StorageError>;
