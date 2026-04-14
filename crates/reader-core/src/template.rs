//! Final article template — askama wrapper + the public
//! [`render_article`] entry point the pipeline calls.

use std::time::Duration;

use askama::Template;

use crate::{
    context::Context,
    pipeline_error::PipelineError,
    text_element::{Header, TextCompound},
};

/// Upper bound on how long `render_article` will wait for a single
/// image worker to finish. Past this, the response goes out even if
/// the `.avif` isn't on disk yet, on the theory that the browser has
/// already given up.
const IMAGE_WAIT_TIMEOUT: Duration = Duration::from_secs(15);

/// Starting capacity for the template body buffer. Most cleaned
/// articles land somewhere between 5 KB and 30 KB; pre-allocating
/// 50 KB avoids repeated `String` regrows during the html compiler walk.
const HTML_BODY_CAPACITY_HINT: usize = 50_000;

#[derive(Template)]
#[template(path = "article.html", escape = "html")]
struct ArticleTemplate<'a> {
    url: &'a str,
    code: &'a str,
    has_code: bool,
    download_link: Option<String>,
}

/// Build the `TextCompound` sequence that seeds the article body: the
/// main `<h1>` with the page title and the `<img>` with the hero
/// image.
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
/// Image re-encoding runs in parallel via the registered image
/// backend. We collect every resulting [`crate::image::ImageTicket`]
/// eagerly (so all workers are launched before we start blocking) and
/// then wait on each one with a bounded timeout before serializing
/// the template.
#[allow(clippy::needless_collect)]
pub fn render_article(parts: &[TextCompound], ctx: &mut Context) -> Result<String, PipelineError> {
    let ctx_snapshot = ctx.clone();
    let header = article_header(&ctx_snapshot);

    let mut body = String::with_capacity(HTML_BODY_CAPACITY_HINT);
    // Collect up-front so every image worker is spawned before we
    // start waiting on any of them.
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
    .map_err(|e| PipelineError::Render(e.to_string()))
}
