/// Every way a diataxis operation can fail.
#[derive(Debug, thiserror::Error)]
#[cfg_attr(feature = "cli", derive(miette::Diagnostic))]
pub enum Error {
    #[error("no diataxis.yml at or above {0}; run `diataxis init` first")]
    NoStore(String),

    #[error("no page matches {0}")]
    NoSuchPage(String),

    #[error("{0} matches more than one page: {1}")]
    AmbiguousPage(String, String),

    #[error("a page named {0} already exists")]
    PageExists(String),

    #[error("{0} is not a Diataxis type; use tutorial, how-to, reference, or explanation")]
    UnknownKind(String),

    #[error("{0}")]
    General(String),

    #[error("{0}")]
    Store(#[from] mdstore::Error),

    #[error("{0}")]
    Yaml(#[from] yaml_serde::Error),

    #[error("{0}")]
    Regex(#[from] regex::Error),

    #[error("{0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
