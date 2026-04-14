use crate::http_error::HttpError;

/// Errors from the image decode + AVIF re-encode path.
#[derive(Debug, thiserror::Error)]
pub enum ImageError {
    #[error("Image decode failed: {0}")]
    Decode(#[from] image::ImageError),

    #[error("AVIF encode failed: {0}")]
    AvifEncode(String),

    #[error("Unsupported image pixel format")]
    UnsupportedFormat,

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Http(#[from] HttpError),
}
