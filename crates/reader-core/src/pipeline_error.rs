use crate::html_node_error::NodeError;
use crate::http_error::HttpError;

/// Errors from the end-to-end article pipeline (`crate::pipeline::render`).
///
/// Aggregates HTTP, HTML-tree and template errors via `#[from]`, plus a
/// handful of pipeline-specific variants that don't belong to any single
/// inner stage.
#[derive(Debug, thiserror::Error)]
pub enum PipelineError {
    #[error(transparent)]
    Http(#[from] HttpError),

    #[error(transparent)]
    Html(#[from] NodeError),

    #[error("Readability extraction failed: {0}")]
    Readability(String),

    #[error("Invalid URL: {0}")]
    InvalidUrl(String),

    #[error("No article content could be extracted")]
    EmptyArticle,

    #[error("Template render failed: {0}")]
    Render(String),

    #[error("Blocking worker panicked")]
    BlockingCanceled,
}
