//! Cross-cutting helpers: HTTP fetching, URL massaging, hashing and the
//! final article template. This is a grab-bag on purpose — splitting it
//! into four one-function files would cost more in import churn than it
//! would buy in clarity.

use std::{borrow::Cow, collections::HashMap, time::Duration};

use once_cell::sync::Lazy;
use reqwest::{
    header::{self, HeaderMap, HeaderValue},
    Url,
};

use crate::{
    config::CONFIG,
    context::Context,
    error::{Error, Result},
    text_element::{Header, TextCompound},
};

// =====================================================================
// Constants
// =====================================================================

/// Canonical forms for tag-like names that appear in HTML variants we want
/// to treat as the standard tag. Used by [`canonical_tag`] to map
/// `amp-img` / `img-responsive` / … back to `img`.
const TAG_ALIASES: &[&str] = &["img", "source"];

/// Attribute value suffixes that suggest the attribute holds a path to an
/// image. Used by [`extract_image_src`] to pick the "real" image URL out of
/// a cluttered `<img>` attribute set.
const IMAGE_SUFFIXES: &[&str] = &[
    ".jpg", ".jpeg", ".webm", ".tiff", ".png", ".bmp", ".gif", ".svg",
];

/// URL suffixes we consider "not an HTML document" when deciding whether
/// to rewrite outbound links through `/m/`.
const NON_HTML_EXTENSIONS: &[&str] = &[
    ".png", ".jpg", ".jpeg", ".gif", ".pdf", ".doc", ".css", ".js", ".bmp", ".webm", ".mp4",
    ".mp3", ".mov", ".tiff", ".tif", ".zip", ".tar", ".gz", ".7z", ".rar", ".py", ".rs", ".c",
    ".xls", ".odt", ".ods", ".wav", ".flac", ".avi", ".m4a", ".json", ".ico", ".ttf", ".woff",
    ".woff2",
];

/// Hard ceiling on image fetch size — 50 MB. Independent of `CONFIG.max_size`
/// (which applies to articles) because a large hero image is sometimes
/// larger than the entire article body we expect.
const IMAGE_SIZE_LIMIT: u64 = 50_000_000;

/// Upper bound on how long `render_article` will wait for a single image
/// worker to finish. Past this, the response goes out even if the `.avif`
/// isn't on disk yet, on the theory that the browser has already given up.
const IMAGE_WAIT_TIMEOUT: Duration = Duration::from_secs(15);

/// Starting capacity for the template body buffer. Most cleaned articles
/// land somewhere between 5 KB and 30 KB; pre-allocating 50 KB avoids
/// repeated `String` regrows during the html compiler walk.
const HTML_BODY_CAPACITY_HINT: usize = 50_000;

// =====================================================================
// URL / tag helpers
// =====================================================================

/// Normalize a tag name. `<amp-img>` and `<picture-source>` both get
/// collapsed down to their canonical (`img`, `source`) aliases so the rest
/// of the pipeline can treat them uniformly.
pub fn canonical_tag(tag: &str) -> &str {
    TAG_ALIASES
        .iter()
        .find(|alias| tag.contains(*alias))
        .unwrap_or(&tag)
}

/// Pick the best candidate image URL out of an `<img>` element's
/// attributes. Ignores the non-URL attributes we know about (`alt`,
/// `class`, `size`, `width`, `height`) and prefers values that look like a
/// path (`/`) or carry an image-file suffix.
///
/// Returns `None` if the extracted value matches the page's already-known
/// hero image — we don't want to emit two copies of the same picture.
pub fn extract_image_src<'a>(
    ctx: &Context,
    attrs: &'a HashMap<String, String>,
) -> Option<Cow<'a, str>> {
    for (name, value) in attrs {
        let looks_like_image =
            value.contains('/') || IMAGE_SUFFIXES.iter().any(|suffix| value.ends_with(suffix));
        let is_url_attr = !matches!(name.as_str(), "alt" | "class" | "size" | "width" | "height");

        if is_url_attr && looks_like_image {
            if let Some(absolute) = absolutize_link(&ctx.url, value) {
                let duplicates_hero = ctx
                    .meta
                    .image
                    .as_ref()
                    .map(|hero| hero == absolute.as_ref())
                    .unwrap_or(false);
                return (!duplicates_hero).then_some(absolute);
            }
        }
    }
    None
}

/// Trim everything after and including `c`, returning the remaining prefix.
fn prefix_before(s: &str, c: char) -> &str {
    &s[..s.find(c).unwrap_or(s.len())]
}

/// If `raw` looks like an `srcset` value (`"img-1x.jpg 1x, img-2x.jpg 2x"`),
/// return just the first URL without its descriptor. Otherwise return `raw`
/// unchanged.
fn strip_srcset_descriptor(raw: &str) -> &str {
    if !raw.contains(' ') {
        return raw;
    }
    let first_candidate = prefix_before(raw, ',').trim();
    prefix_before(first_candidate, ' ')
}

/// Resolve a link against a base URL. Passes absolute `http(s)` URLs
/// through, rejects `data:` URIs, and uses `Url::join` for anything else.
/// Also strips `srcset` descriptors — real-world `<img>` tags hide their
/// actual URL inside strings like `"hero-1x.jpg 1x, hero-2x.jpg 2x"`.
pub fn absolutize_link<'a>(base: &Url, raw: &'a str) -> Option<Cow<'a, str>> {
    let raw = strip_srcset_descriptor(raw);
    if raw.starts_with("data") {
        None
    } else if raw.starts_with("http") {
        Some(Cow::Borrowed(raw))
    } else {
        Some(Cow::Owned(base.join(raw).ok()?.to_string()))
    }
}

pub fn is_html(url: &str) -> bool {
    !NON_HTML_EXTENSIONS
        .iter()
        .any(|suffix| url.ends_with(suffix))
}

// =====================================================================
// Hashing
// =====================================================================

pub fn sha256(data: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(data.as_bytes());
    hex::encode(hasher.finalize())
}

// =====================================================================
// HTTP
// =====================================================================

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
pub fn http_get_bytes(url: &str) -> Result<Vec<u8>> {
    let resp = BLOCKING_CLIENT.get(url).send()?;
    if resp.content_length().unwrap_or(0) > IMAGE_SIZE_LIMIT {
        return Err(Error::ResponseTooLarge {
            limit: IMAGE_SIZE_LIMIT,
        });
    }
    let bytes = resp.bytes()?;
    if bytes.len() as u64 > IMAGE_SIZE_LIMIT {
        return Err(Error::ResponseTooLarge {
            limit: IMAGE_SIZE_LIMIT,
        });
    }
    Ok(bytes.to_vec())
}

/// Async article fetch — used by the main pipeline. Decodes `iso-8859-1`
/// declared pages byte-for-byte (we don't get perfect codepoint mapping
/// but we avoid `lossy` replacement for the first 256 code points) and
/// returns `lossy` UTF-8 otherwise.
pub async fn http_get(url: &str) -> Result<String> {
    let resp = ASYNC_CLIENT.get(url).send().await?;
    if resp.content_length().unwrap_or(0) > CONFIG.max_size {
        return Err(Error::ResponseTooLarge {
            limit: CONFIG.max_size,
        });
    }
    let bytes = resp.bytes().await?;
    if bytes.len() as u64 > CONFIG.max_size {
        return Err(Error::ResponseTooLarge {
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

// =====================================================================
// Template
// =====================================================================

#[derive(askama::Template)]
#[template(path = "article.html", escape = "html")]
struct ArticleTemplate<'a> {
    url: &'a str,
    code: &'a str,
    has_code: bool,
    download_link: Option<String>,
}

/// Build the `TextCompound` sequence that seeds the article body: the
/// main `<h1>` with the page title and the `<img>` with the hero image.
fn article_header<'a>(ctx: &'a Context<'a>) -> [TextCompound<'a>; 2] {
    let title = ctx.meta.title.as_deref().unwrap_or("");
    let image = ctx.meta.image.as_deref().unwrap_or("");
    [
        TextCompound::heading(Header::H1, ["main-title"], TextCompound::raw(title)),
        TextCompound::img(image),
    ]
}

/// Compile a sequence of `TextCompound` parts into the final HTML
/// response, wrapping it in the askama template at
/// `templates/article.html`.
///
/// Image re-encoding runs in parallel via the registered image backend.
/// We collect every resulting [`crate::image::ImageTicket`] eagerly (so
/// all workers are launched before we start blocking) and then wait on
/// each one with a bounded timeout before serializing the template.
#[allow(clippy::needless_collect)]
pub fn render_article(parts: &[TextCompound], ctx: &mut Context) -> Result<String> {
    use askama::Template;

    let ctx_snapshot = ctx.clone();
    let header = article_header(&ctx_snapshot);

    let mut body = String::with_capacity(HTML_BODY_CAPACITY_HINT);
    // Collect up-front so every image worker is spawned before we start
    // waiting on any of them; the comment on `#[allow(clippy::needless_collect)]`
    // above explains why we can't fuse this into a lazy `.for_each`.
    let tickets: Vec<_> = header
        .iter()
        .chain(parts.iter())
        .flat_map(|node| node.html(ctx, &mut body))
        .collect();
    for ticket in tickets {
        let _ = ticket.done.recv_timeout(IMAGE_WAIT_TIMEOUT);
    }

    let download_link = (!ctx.mode.is_download()).then(|| format!("/d/{}", ctx.min_id));
    let has_code = body.contains("<code>");
    ArticleTemplate {
        url: ctx.url.as_str(),
        code: &body,
        has_code,
        download_link,
    }
    .render()
    .map_err(|e| Error::Render(e.to_string()))
}
