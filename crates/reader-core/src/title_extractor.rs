//! Regex-based scanner for the handful of metadata fields we care about.
//!
//! Rather than run a full html5ever parse just to grab `og:title`, we scan
//! the `<head>` region of the raw HTML with a pair of meta-tag regexes and
//! a `<title>` regex. This saves one full DOM construction per request.

use once_cell::sync::Lazy;
use regex::Regex;

/// Metadata we extract from a page before running Readability.
///
/// `title` / `image` come from Open Graph–style `<meta property=…>` tags;
/// `html_title` is the text of the `<title>` element, used as a fallback
/// when the page has no og:title and Readability doesn't guess one either.
#[derive(Default, Debug, Clone)]
pub struct ArticleData {
    pub image: Option<String>,
    pub title: Option<String>,
    pub html_title: Option<String>,
}

const TITLE_PROPERTIES: &[&str] = &["og:title", "title", "twiter:title", "discord:title"];
const IMAGE_PROPERTIES: &[&str] = &["og:image", "image", "twiter:image", "discord:image"];

const HEAD_CLOSE: &str = "</head>";

/// Cap on how much raw HTML to scan when the page has no `</head>`
/// marker. Scanning the whole response on a badly-formed document would
/// cost more than the html5ever parse we're trying to avoid.
const METADATA_SCAN_FALLBACK: usize = 64 * 1024;

// Matches `<meta … property="…" … content="…" …>` in either attribute order.
static META_PROP_CONTENT: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r#"(?is)<meta\s+[^>]*property\s*=\s*["']([^"']+)["'][^>]*content\s*=\s*["']([^"']*)["']"#,
    )
    .unwrap()
});
static META_CONTENT_PROP: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r#"(?is)<meta\s+[^>]*content\s*=\s*["']([^"']*)["'][^>]*property\s*=\s*["']([^"']+)["']"#,
    )
    .unwrap()
});
static TITLE_TAG: Lazy<Regex> =
    Lazy::new(|| Regex::new(r#"(?is)<title[^>]*>([^<]*)</title>"#).unwrap());

/// Scan raw HTML for Open Graph metadata and the `<title>` tag.
pub fn try_extract_data(html: &str) -> ArticleData {
    let head = head_region(html);

    let html_title = TITLE_TAG
        .captures(head)
        .and_then(|caps| caps.get(1))
        .map(|m| decode(m.as_str().trim()));

    let mut title = None;
    let mut image = None;

    scan_meta(head, &META_PROP_CONTENT, 1, 2, &mut title, &mut image);
    if title.is_none() || image.is_none() {
        scan_meta(head, &META_CONTENT_PROP, 2, 1, &mut title, &mut image);
    }

    ArticleData {
        html_title,
        title,
        image,
    }
}

/// Slice the raw HTML down to the `<head>` region (plus the closing tag)
/// so the regex scans have bounded input.
fn head_region(html: &str) -> &str {
    let end = html
        .find(HEAD_CLOSE)
        .map(|i| i + HEAD_CLOSE.len())
        .unwrap_or_else(|| html.len().min(METADATA_SCAN_FALLBACK));
    &html[..end]
}

/// Walk a single meta regex over `head`, filling `title` / `image` with
/// the first matching og:* values. `prop_group` / `content_group` name the
/// capture indices for the property and content values respectively,
/// because the two regexes differ in attribute order.
fn scan_meta(
    head: &str,
    regex: &Regex,
    prop_group: usize,
    content_group: usize,
    title: &mut Option<String>,
    image: &mut Option<String>,
) {
    for caps in regex.captures_iter(head) {
        let prop = &caps[prop_group];
        let content = &caps[content_group];
        if title.is_none() && TITLE_PROPERTIES.contains(&prop) {
            *title = Some(decode(content));
        } else if image.is_none() && IMAGE_PROPERTIES.contains(&prop) {
            *image = Some(decode(content));
        }
        if title.is_some() && image.is_some() {
            return;
        }
    }
}

fn decode(s: &str) -> String {
    html_escape::decode_html_entities(s).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pulls_og_title_and_image_in_either_attribute_order() {
        let html = r#"
            <html><head>
              <title>Fallback Title</title>
              <meta property="og:title" content="Real Title">
              <meta content="https://example.com/hero.jpg" property="og:image">
            </head><body>x</body></html>
        "#;
        let data = try_extract_data(html);
        assert_eq!(data.title.as_deref(), Some("Real Title"));
        assert_eq!(data.image.as_deref(), Some("https://example.com/hero.jpg"));
        assert_eq!(data.html_title.as_deref(), Some("Fallback Title"));
    }

    #[test]
    fn decodes_html_entities_in_content() {
        let html = r#"<head><meta property="og:title" content="Cats &amp; Dogs"></head>"#;
        let data = try_extract_data(html);
        assert_eq!(data.title.as_deref(), Some("Cats & Dogs"));
    }

    #[test]
    fn missing_metadata_yields_none() {
        let html = "<html><head></head><body>nothing here</body></html>";
        let data = try_extract_data(html);
        assert!(data.title.is_none());
        assert!(data.image.is_none());
    }

    #[test]
    fn ignores_non_matching_meta_properties() {
        let html = r#"<head><meta property="description" content="blurb"></head>"#;
        let data = try_extract_data(html);
        assert!(data.title.is_none());
    }
}
