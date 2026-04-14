use image_actor::ImageActorError;
use page_actor::PageActorError;
use reader_core::CacheError;

/// Top-level error type for the HTTP binding. Aggregates the narrow
/// errors returned by each downstream layer via `#[from]`; each handler
/// converts it into an HTTP status in `response_for_error`.
#[derive(Debug, thiserror::Error)]
pub enum ServerError {
    #[error(transparent)]
    Cache(#[from] CacheError),

    #[error(transparent)]
    PageActor(#[from] PageActorError),

    #[error(transparent)]
    ImageActor(#[from] ImageActorError),

    #[error("Invalid base64 payload: {0}")]
    Base64(#[from] base64::DecodeError),

    #[error("Invalid UTF-8 payload: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),

    #[error("Short id not found in database")]
    UnknownShortId,

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}
