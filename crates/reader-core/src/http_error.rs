/// Errors that can come out of the HTTP fetch path.
#[derive(Debug, thiserror::Error)]
pub enum HttpError {
    #[error("HTTP request failed: {0}")]
    Request(#[from] reqwest::Error),

    #[error("Response body exceeded {limit} bytes")]
    TooLarge { limit: u64 },
}
