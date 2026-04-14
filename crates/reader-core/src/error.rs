//! Crate-root error aggregator.
//!
//! Each module in reader-core has its own narrow error type
//! (`HttpError`, `NodeError`, `CacheError`, `ImageError`, `PipelineError`).
//! This module's `Error` is the union of those — downstream crates that
//! want a single error type to `?` through can use this one, while
//! functions internal to the module hierarchy return their narrower
//! types.

use crate::cache_error::CacheError;
use crate::html_node_error::NodeError;
use crate::http_error::HttpError;
use crate::image::ImageError;
use crate::pipeline_error::PipelineError;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Http(#[from] HttpError),

    #[error(transparent)]
    Node(#[from] NodeError),

    #[error(transparent)]
    Cache(#[from] CacheError),

    #[error(transparent)]
    Image(#[from] ImageError),

    #[error(transparent)]
    Pipeline(#[from] PipelineError),

    #[error("Blocking worker panicked")]
    BlockingCanceled,
}
