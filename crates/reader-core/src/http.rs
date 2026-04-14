//! HTTP fetch helpers — shared async + blocking reqwest clients plus
//! the `http_get` / `http_get_bytes` entry points used by the pipeline
//! and the image worker respectively.

use once_cell::sync::Lazy;
use reqwest::header::{self, HeaderMap, HeaderValue};

use crate::{config::CONFIG, http_error::HttpError};

/// Hard ceiling on image fetch size — 50 MB. Independent of
/// `CONFIG.max_size` (which applies to articles) because a large hero
/// image is sometimes larger than the entire article body we expect.
const IMAGE_SIZE_LIMIT: u64 = 50_000_000;

fn default_headers() -> HeaderMap {
    let mut h = HeaderMap::new();
    h.insert(
        header::USER_AGENT,
        HeaderValue::from_static(
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.164 Safari/537.36",
        ),
    );
    h.insert(
        header::ACCEPT_LANGUAGE,
        HeaderValue::from_static("fr-FR,fr;q=0.9,en-US;q=0.8,en;q=0.7"),
    );
    h.insert(
        header::ACCEPT_ENCODING,
        HeaderValue::from_static("gzip, deflate"),
    );
    h.insert(
        header::ACCEPT,
        HeaderValue::from_static(
            "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8,application/signed-exchange;v=b3;q=0.9",
        ),
    );
    h
}

static ASYNC_CLIENT: Lazy<reqwest::Client> = Lazy::new(|| {
    reqwest::Client::builder()
        .cookie_store(true)
        .default_headers(default_headers())
        .build()
        .expect("failed to build async reqwest client")
});

static BLOCKING_CLIENT: Lazy<reqwest::blocking::Client> = Lazy::new(|| {
    reqwest::blocking::Client::builder()
        .cookie_store(true)
        .default_headers(default_headers())
        .build()
        .expect("failed to build blocking reqwest client")
});

fn latin1_to_string(bytes: &[u8]) -> String {
    bytes.iter().copied().map(char::from).collect()
}

/// Blocking image fetch — called from the std::thread image-actor worker.
pub fn http_get_bytes(url: &str) -> Result<Vec<u8>, HttpError> {
    let resp = BLOCKING_CLIENT.get(url).send()?;
    if resp.content_length().unwrap_or(0) > IMAGE_SIZE_LIMIT {
        return Err(HttpError::TooLarge {
            limit: IMAGE_SIZE_LIMIT,
        });
    }
    let bytes = resp.bytes()?;
    if bytes.len() as u64 > IMAGE_SIZE_LIMIT {
        return Err(HttpError::TooLarge {
            limit: IMAGE_SIZE_LIMIT,
        });
    }
    Ok(bytes.to_vec())
}

/// Async article fetch — used by the main pipeline. Decodes
/// `iso-8859-1` declared pages byte-for-byte (we don't get perfect
/// codepoint mapping but we avoid `lossy` replacement for the first
/// 256 code points) and returns `lossy` UTF-8 otherwise.
pub async fn http_get(url: &str) -> Result<String, HttpError> {
    let resp = ASYNC_CLIENT.get(url).send().await?;
    if resp.content_length().unwrap_or(0) > CONFIG.max_size {
        return Err(HttpError::TooLarge {
            limit: CONFIG.max_size,
        });
    }
    let bytes = resp.bytes().await?;
    if bytes.len() as u64 > CONFIG.max_size {
        return Err(HttpError::TooLarge {
            limit: CONFIG.max_size,
        });
    }
    let lossy = String::from_utf8_lossy(&bytes);
    let head = &lossy[..lossy.find("</head>").unwrap_or(lossy.len())];
    if head.contains("iso-8859-1") {
        Ok(latin1_to_string(&bytes))
    } else {
        Ok(lossy.into_owned())
    }
}
