//! Core reader pipeline: HTML parsing, Readability extraction, image
//! re-encoding scaffolding, template rendering and the on-disk URL cache.
//!
//! This crate is the pure "what the server does" layer. It knows nothing
//! about the actor runtime — the services that wrap it (page-actor,
//! image-actor) and the HTTP binding (server) live in sibling crates.
//!
//! Error types are **per-module**: each feature area exposes its own
//! narrow error enum (`HttpError`, `NodeError`, `CacheError`,
//! `ImageError`, `PipelineError`), and the crate-root [`Error`] unions
//! them via `#[from]` for callers that want a single aggregate type.

pub mod cache;
pub mod cache_error;
pub mod config;
pub mod context;
pub mod error;
pub mod html_node;
pub mod html_node_error;
pub mod http_error;
pub mod image;
pub mod pipeline;
pub mod pipeline_error;
pub mod render_mode;
pub mod score_implementation;
pub mod text_element;
pub mod title_extractor;
pub mod utils;

pub use cache_error::CacheError;
pub use error::{Error, Result};
pub use html_node_error::NodeError;
pub use http_error::HttpError;
pub use pipeline_error::PipelineError;
pub use render_mode::RenderMode;
