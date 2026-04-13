use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Response body exceeded {limit} bytes")]
    ResponseTooLarge { limit: u64 },

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Invalid base64 payload: {0}")]
    Base64(#[from] base64::DecodeError),

    #[error("Invalid UTF-8 payload: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),

    #[error("Invalid URL: {0}")]
    InvalidUrl(String),

    #[error("No article content could be extracted")]
    EmptyArticle,

    #[error("Readability extraction failed: {0}")]
    Readability(String),

    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("URL store mutex poisoned")]
    DbPoisoned,

    #[error("Template render failed: {0}")]
    Render(String),

    #[error("Short id not found in database")]
    UnknownShortId,

    #[error("Skipping <{tag}>: tag is in the blocklist")]
    BlockedTag { tag: String },

    #[error("Skipping <{tag}>: no children")]
    EmptyNode { tag: String },

    #[error("Skipping empty text node")]
    EmptyText,

    #[error("Skipping comment node")]
    CommentNode,

    #[error("Image decode failed: {0}")]
    ImageDecode(#[from] image::ImageError),

    #[error("AVIF encode failed: {0}")]
    AvifEncode(String),

    #[error("Unsupported image pixel format")]
    UnsupportedImageFormat,

    #[error("Blocking worker panicked")]
    BlockingCanceled,
}

impl From<actix_web::error::BlockingError> for Error {
    fn from(_: actix_web::error::BlockingError) -> Self {
        Error::BlockingCanceled
    }
}
