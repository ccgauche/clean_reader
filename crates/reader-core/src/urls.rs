//! URL and tag-name helpers used across the pipeline stages.

use std::{borrow::Cow, collections::HashMap};

use reqwest::Url;

use crate::context::Context;

/// Canonical forms for tag-like names that appear in HTML variants we
/// want to treat as the standard tag. Used by [`canonical_tag`] to map
/// `amp-img` / `img-responsive` / … back to `img`.
const TAG_ALIASES: &[&str] = &["img", "source"];

/// Attribute value suffixes that suggest the attribute holds a path to
/// an image. Used by [`extract_image_src`] to pick the "real" image URL
/// out of a cluttered `<img>` attribute set.
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

/// Normalize a tag name. `<amp-img>` and `<picture-source>` both get
/// collapsed down to their canonical (`img`, `source`) aliases so the
/// rest of the pipeline can treat them uniformly.
pub fn canonical_tag(tag: &str) -> &str {
    TAG_ALIASES
        .iter()
        .find(|alias| tag.contains(*alias))
        .unwrap_or(&tag)
}

/// Pick the best candidate image URL out of an `<img>` element's
/// attributes. Ignores the non-URL attributes we know about (`alt`,
/// `class`, `size`, `width`, `height`) and prefers values that look
/// like a path (`/`) or carry an image-file suffix.
///
/// Returns `None` if the extracted value matches the page's
/// already-known hero image — we don't want to emit two copies of the
/// same picture.
pub fn extract_image_src<'a>(
    ctx: &Context,
    attrs: &'a HashMap<String, String>,
) -> Option<Cow<'a, str>> {
    attrs
        .iter()
        .filter(|(name, _)| is_url_attr(name))
        .filter(|(_, value)| looks_like_image(value))
        .find_map(|(_, value)| {
            let absolute = absolutize_link(&ctx.url, value)?;
            let duplicates_hero = ctx
                .meta
                .image
                .as_deref()
                .is_some_and(|hero| hero == absolute.as_ref());
            (!duplicates_hero).then_some(absolute)
        })
}

fn is_url_attr(name: &str) -> bool {
    !matches!(name, "alt" | "class" | "size" | "width" | "height")
}

fn looks_like_image(value: &str) -> bool {
    value.contains('/') || IMAGE_SUFFIXES.iter().any(|suffix| value.ends_with(suffix))
}

/// Trim everything after and including `c`, returning the remaining prefix.
fn prefix_before(s: &str, c: char) -> &str {
    &s[..s.find(c).unwrap_or(s.len())]
}

/// If `raw` looks like an `srcset` value
/// (`"img-1x.jpg 1x, img-2x.jpg 2x"`), return just the first URL
/// without its descriptor. Otherwise return `raw` unchanged.
fn strip_srcset_descriptor(raw: &str) -> &str {
    if !raw.contains(' ') {
        return raw;
    }
    let first_candidate = prefix_before(raw, ',').trim();
    prefix_before(first_candidate, ' ')
}

/// Resolve a link against a base URL. Passes absolute `http(s)` URLs
/// through, rejects `data:` URIs, and uses `Url::join` for anything
/// else. Also strips `srcset` descriptors — real-world `<img>` tags
/// hide their actual URL inside strings like
/// `"hero-1x.jpg 1x, hero-2x.jpg 2x"`.
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

/// Whether `url` ends with an extension that suggests it's not an HTML
/// document. Used to decide whether to rewrite an outbound link through
/// `/m/{short}`.
pub fn is_html(url: &str) -> bool {
    !NON_HTML_EXTENSIONS
        .iter()
        .any(|suffix| url.ends_with(suffix))
}
