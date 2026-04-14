//! End-to-end article pipeline.
//!
//! [`render`] is the public entry point: given a URL it fetches the HTML
//! (following `amphtml` hints where present), hands the body to Readability
//! for content selection, then lowers the result through `HTMLNode`,
//! `TextCompound` and the askama template to produce the final page.
//!
//! All CPU-bound work runs inside `spawn_blocking`; only the network
//! fetches touch the async executor directly.

use std::collections::HashMap;
use std::io::Cursor;

use html5ever::tendril::TendrilSink;

use crate::{
    context::Context, html_node::HTMLNode, http, pipeline_error::PipelineError,
    render_mode::RenderMode, score_implementation::starts_with_image, template::render_article,
    text_element::TextCompound, title_extractor,
};

type Result<T> = std::result::Result<T, PipelineError>;

/// Fetch a URL and render it through the reader pipeline.
pub async fn render(url: &str, min_id: &str, mode: RenderMode) -> Result<String> {
    let html = fetch_with_amp_fallback(url).await?;
    let parsed_url =
        reqwest::Url::parse(url).map_err(|e| PipelineError::InvalidUrl(e.to_string()))?;
    let min_id = min_id.to_string();
    tokio::task::spawn_blocking(move || render_fetched_html(html, parsed_url, min_id, mode))
        .await
        .map_err(|_| PipelineError::BlockingCanceled)?
}

/// Download the article HTML, replacing it with the AMP version if one is
/// linked and reachable. A malformed link or failed AMP fetch falls back
/// to the original HTML rather than erroring.
async fn fetch_with_amp_fallback(url: &str) -> Result<String> {
    let original = http::http_get(url).await?;
    let Some(amp_url) = extract_amp_url(&original) else {
        return Ok(original);
    };
    eprintln!("Using AMPHTML: {}", amp_url);
    Ok(http::http_get(&amp_url).await.unwrap_or(original))
}

/// Scan raw HTML for a `rel="amphtml"` link and return its target.
fn extract_amp_url(html: &str) -> Option<String> {
    const MARKER: &str = "rel=\"amphtml\"";
    let marker_start = html.find(MARKER)?;
    let after = &html[marker_start + MARKER.len()..];
    after.split('"').nth(1).map(str::to_owned)
}

/// CPU-bound half of the pipeline: Readability → `HTMLNode` →
/// `TextCompound` → askama template. Runs inside `spawn_blocking`.
fn render_fetched_html(
    html: String,
    parsed_url: reqwest::Url,
    min_id: String,
    mode: RenderMode,
) -> Result<String> {
    // Lightweight regex scan for og:title / og:image / <title>, avoiding a
    // full html5ever parse just for metadata.
    let mut meta = title_extractor::try_extract_data(&html);

    // Readability (Firefox reader-view algorithm) picks the article
    // subtree and returns it as a serialized HTML fragment.
    let product = readability::extractor::extract(&mut Cursor::new(html), &parsed_url)
        .map_err(|e| PipelineError::Readability(e.to_string()))?;
    if meta.title.is_none() && !product.title.is_empty() {
        meta.title = Some(product.title);
    }

    // Parse the cleaned fragment into our `HTMLNode` tree.
    let content_dom = html5ever::parse_document(
        markup5ever_rcdom::RcDom::default(),
        html5ever::ParseOpts::default(),
    )
    .one(product.content);
    let html_tree = HTMLNode::from_handle(&content_dom.document)?;

    let mut ctx = Context {
        meta,
        mode,
        min_id,
        url: parsed_url,
        map: HashMap::new(),
        count: 0,
    };
    let article =
        TextCompound::from_node(&mut ctx, &html_tree).ok_or(PipelineError::EmptyArticle)?;

    // Drop a redundant leading H1 if we already have a page title.
    let article = if ctx.meta.title.is_some() {
        article.remove_title()
    } else {
        article
    };

    // Fall back to <title> text if the page supplied neither og:title nor
    // a Readability guess.
    if ctx.meta.title.is_none() && !article.contains_title() {
        ctx.meta.title = ctx.meta.html_title.clone();
    }

    // Suppress the hero metadata image if the article body already starts
    // with an image — otherwise we'd show two of the same picture.
    if starts_with_image(&html_tree) {
        ctx.meta.image = None;
    }

    render_article(&[article], &mut ctx)
}
