//! Core reader pipeline: HTML parsing, Readability extraction, image
//! re-encoding scaffolding, template rendering and the on-disk URL cache.
//!
//! This crate is the pure "what the server does" layer. It knows nothing
//! about the actor runtime — the services that wrap it (page-actor,
//! image-actor) and the HTTP binding (server) live in sibling crates.
//!
//! Cross-crate wiring uses the function-pointer registry in
//! [`image::register_encoder`]: reader-core calls through a registered
//! closure when the template renderer needs an image re-encoded, so the
//! image-actor crate can supply the backend at boot without creating a
//! cyclic crate dependency.

pub mod cache;
pub mod config;
pub mod error;
pub mod html_node;
pub mod image;
pub mod pipeline;
pub mod score_implementation;
pub mod text_element;
pub mod text_parser;
pub mod title_extractor;
pub mod utils;

pub use error::{Error, Result};
pub use text_parser::RenderMode;
