use once_cell::sync::Lazy;
use regex::Regex;

#[derive(Default, Debug, Clone)]
pub struct ArticleData {
    pub image: Option<String>,
    pub title: Option<String>,
    pub etitle: Option<String>,
}

const TITLE_PROPERTIES: &[&str] = &["og:title", "title", "twiter:title", "discord:title"];
const IMAGE_PROPERTIES: &[&str] = &["og:image", "image", "twiter:image", "discord:image"];

// Matches `<meta ... property="..." ... content="..." ...>` in either attribute order.
// Limited to the pre-`</head>` section by the caller so we don't scan full articles.
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

/// Scans raw HTML for the metadata we care about — Open Graph-style
/// `<meta property="…" content="…">` and the `<title>` tag. Bypasses a second
/// html5ever parse: Readability re-parses internally, so we only pay the
/// cost of its pass plus a handful of regex runs here.
pub fn try_extract_data(html: &str) -> ArticleData {
    // Only scan the <head> region (plus some slack) to keep the work bounded.
    let scan_end = html
        .find("</head>")
        .map(|i| i + "</head>".len())
        .unwrap_or_else(|| html.len().min(64 * 1024));
    let head = &html[..scan_end];

    let etitle = TITLE_TAG
        .captures(head)
        .and_then(|c| c.get(1))
        .map(|m| decode_html_entities(m.as_str().trim()));

    let mut title = None;
    let mut image = None;

    for cap in META_PROP_CONTENT.captures_iter(head) {
        let prop = &cap[1];
        let content = cap[2].to_owned();
        assign_if_match(&mut title, &mut image, prop, content);
        if title.is_some() && image.is_some() {
            break;
        }
    }
    if title.is_none() || image.is_none() {
        for cap in META_CONTENT_PROP.captures_iter(head) {
            let content = cap[1].to_owned();
            let prop = &cap[2];
            assign_if_match(&mut title, &mut image, prop, content);
            if title.is_some() && image.is_some() {
                break;
            }
        }
    }

    ArticleData {
        etitle,
        title,
        image,
    }
}

fn assign_if_match(
    title: &mut Option<String>,
    image: &mut Option<String>,
    prop: &str,
    content: String,
) {
    if title.is_none() && TITLE_PROPERTIES.contains(&prop) {
        *title = Some(decode_html_entities(&content));
    } else if image.is_none() && IMAGE_PROPERTIES.contains(&prop) {
        *image = Some(decode_html_entities(&content));
    }
}

fn decode_html_entities(s: &str) -> String {
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
        assert_eq!(data.etitle.as_deref(), Some("Fallback Title"));
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
