use std::collections::HashMap;
use std::io::Cursor;

use html5ever::tendril::TendrilSink;

use crate::{
    error::{Error, Result},
    html_node::HTMLNode,
    score_implementation::contains_image,
    text_element::TextCompound,
    utils::{self, gen_html_2},
};

/// Fetch a URL (following AMP redirects when present) and render it through
/// the clean-read pipeline. Async so we can use the shared reqwest client;
/// the CPU-bound stages run inside `spawn_blocking` to avoid stalling the
/// async executor.
pub async fn run_v2(url: &str, min_id: &str, as_download: bool) -> Result<String> {
    let httpstring = utils::http_get(url).await?;
    // Look for a `rel="amphtml"` link and, if present, fetch the AMP version.
    // A malformed link or a failed fetch falls back to the original HTML.
    let amp_target = httpstring
        .find("rel=\"amphtml\"")
        .and_then(|i| {
            httpstring[(i + "rel=\"amphtml\"".len())..]
                .split('"')
                .nth(1)
        })
        .map(str::to_owned);
    let httpstring = if let Some(amp_url) = amp_target {
        println!("Using AMPHTML");
        utils::http_get(&amp_url).await.unwrap_or(httpstring)
    } else {
        httpstring
    };
    let parsed_url = reqwest::Url::parse(url).map_err(|e| Error::InvalidUrl(e.to_string()))?;
    let min_id = min_id.to_string();
    tokio::task::spawn_blocking(move || render(httpstring, parsed_url, min_id, as_download))
        .await
        .map_err(|_| Error::BlockingCanceled)?
}

fn render(
    httpstring: String,
    parsed_url: reqwest::Url,
    min_id: String,
    as_download: bool,
) -> Result<String> {
    // Scan metadata from the raw HTML — cheap regex over the <head> region,
    // avoids an extra full html5ever parse.
    let mut meta = crate::title_extractor::try_extract_data(&httpstring);

    // Readability (Firefox reader-view algorithm) picks the article subtree.
    let mut cursor = Cursor::new(httpstring);
    let product = readability::extractor::extract(&mut cursor, &parsed_url)
        .map_err(|e| Error::Readability(e.to_string()))?;
    if meta.title.is_none() && !product.title.is_empty() {
        meta.title = Some(product.title);
    }

    // Lower Readability's cleaned HTML into our HTMLNode / TextCompound pipeline.
    let content_dom = html5ever::parse_document(
        markup5ever_rcdom::RcDom::default(),
        html5ever::ParseOpts::default(),
    )
    .one(product.content);
    let html = HTMLNode::from_handle(&content_dom.document)?;

    let mut ctx = crate::text_parser::Context {
        meta,
        download: as_download,
        min_id,
        url: parsed_url,
        map: HashMap::new(),
        count: 0,
    };
    let text = TextCompound::from_node(&mut ctx, &html).ok_or(Error::EmptyArticle)?;
    let text = if ctx.meta.title.is_some() {
        text.remove_title()
    } else {
        text
    };
    if ctx.meta.title.is_none() && !text.contains_title() {
        ctx.meta.title = ctx.meta.etitle.clone();
    };
    if contains_image(&html).0 {
        ctx.meta.image = None;
    }
    gen_html_2(&[text], &mut ctx)
}
