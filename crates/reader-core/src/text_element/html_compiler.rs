//! Walk a [`TextCompound`] tree and emit the final article HTML fragment.

use std::ops::Not;

use crate::{
    cache::get_shortened_from_url,
    context::Context,
    image::{get_image_url, ImageTicket},
    text_element::TextCompound,
    utils::is_html,
};

const PUNCTUATION: &str = ".,;:!?()[]{}";

impl<'a> TextCompound<'a> {
    /// Render `self` into `out`, returning the image tickets spawned along
    /// the way. The caller is expected to block on all returned tickets
    /// before finalising the response.
    pub fn html(&'a self, ctx: &mut Context, out: &mut String) -> Vec<ImageTicket> {
        match self {
            Self::Raw(a) => {
                // If the raw text starts with punctuation, eat the trailing
                // space from whatever was just emitted so the result reads
                // like `sentence, foo` rather than `sentence , foo`.
                if let Some(first) = a.chars().next() {
                    if PUNCTUATION.contains(first) {
                        match out.pop() {
                            Some(' ') | None => (),
                            Some(restored) => out.push(restored),
                        }
                    }
                }
                out.push_str(&html_escape::encode_text(a));
                vec![]
            }
            Self::Link { content, href } => {
                let rewritten = rewrite_href(ctx, href);
                push_element(out, "a", Some(("href".to_owned(), rewritten)), content, ctx)
            }
            Self::Italic(child) => push_simple_element(out, "i", child, ctx),
            Self::Bold(child) => push_simple_element(out, "b", child, ctx),
            Self::Underline(child) => push_simple_element(out, "u", child, ctx),
            Self::Array(items) => items.iter().flat_map(|x| x.html(ctx, out)).collect(),
            Self::Abbr { content, title } => {
                push_element(out, "small", Some(("title", title.as_ref())), content, ctx)
            }
            Self::Sup(child) => push_simple_element(out, "sup", child, ctx),
            Self::Sub(child) => push_simple_element(out, "sub", child, ctx),
            Self::Small(child) => push_simple_element(out, "small", child, ctx),
            Self::Br => {
                out.push_str("<br/>");
                vec![]
            }
            Self::Code(text) => {
                if text.contains('\n') {
                    push_container(out, "pre", |out| {
                        push_container(out, "code", |out| {
                            out.push_str(&html_escape::encode_text(text));
                            vec![]
                        })
                    })
                } else {
                    push_container(out, "code", |out| {
                        out.push_str(&html_escape::encode_text(text));
                        out.push_str("&nbsp;");
                        vec![]
                    })
                }
            }
            Self::Img(src) => {
                let (rewritten, ticket) = get_image_url(src);
                out.push_str("<img src=\"");
                out.push_str(&rewritten);
                out.push_str("\">");
                ticket.map(|t| vec![t]).unwrap_or_default()
            }
            Self::Heading {
                fragment_ids,
                level,
                content,
            } => {
                let rewritten_ids: Vec<_> = fragment_ids
                    .iter()
                    .flat_map(|id| ctx.map.get(id.as_ref()))
                    .map(|n| format!("#{}", n))
                    .collect();
                let attr = rewritten_ids
                    .is_empty()
                    .not()
                    .then(|| ("id".to_string(), rewritten_ids.join(" ")));
                push_element(out, level.to_str(), attr, content, ctx)
            }
            Self::Ul(items) => push_container(out, "il", |out| {
                items
                    .iter()
                    .flat_map(|item| push_simple_element(out, "li", item, ctx))
                    .collect()
            }),
            Self::P(child) => push_simple_element(out, "p", child, ctx),
            Self::Table(table) => push_container(out, "table", |out| {
                table
                    .rows
                    .iter()
                    .flat_map(|row| {
                        push_container(out, "tr", |out| {
                            row.cells
                                .iter()
                                .flat_map(|cell| {
                                    push_simple_element(out, cell.html_tag(), cell.content(), ctx)
                                })
                                .collect()
                        })
                    })
                    .collect()
            }),
            Self::Quote(child) => push_simple_element(out, "quote", child, ctx),
        }
    }
}

/// Decide what href to emit for a link. In View mode we route outbound
/// HTTP(S) links through `/m/{short}` for one-click cleaning; in Download
/// mode and for mailto / fragment / non-HTML links we pass the href
/// through unchanged. A failed cache write is logged and falls back to the
/// original href rather than propagating.
fn rewrite_href(ctx: &Context, raw: &str) -> String {
    let rewritable = !ctx.mode.is_download()
        && !raw.starts_with("mailto:")
        && !raw.starts_with('#')
        && is_html(raw);
    if !rewritable {
        return raw.to_string();
    }
    match get_shortened_from_url(raw) {
        Ok(short) => format!("/m/{}", short),
        Err(e) => {
            eprintln!("link shorten failed for {}: {}", raw, e);
            raw.to_string()
        }
    }
}

/// Write `<tag>child</tag>` with no attributes. Returns the image tickets
/// the child spawned.
fn push_simple_element(
    out: &mut String,
    tag: &str,
    child: &TextCompound,
    ctx: &mut Context,
) -> Vec<ImageTicket> {
    push_element::<String>(out, tag, None, child, ctx)
}

/// Write `<tag>…</tag>` where the body is built by `build`. Returns
/// whatever `build` returns.
fn push_container(
    out: &mut String,
    tag: &str,
    build: impl FnOnce(&mut String) -> Vec<ImageTicket>,
) -> Vec<ImageTicket> {
    wrap_tag(out, tag, None as Option<(String, String)>, build)
}

/// Write `<tag attr="value">…</tag>` wrapping a `TextCompound` child.
fn push_element<T: Into<String>>(
    out: &mut String,
    tag: &str,
    attribute: Option<(T, T)>,
    child: &TextCompound,
    ctx: &mut Context,
) -> Vec<ImageTicket> {
    wrap_tag(out, tag, attribute, |out| child.html(ctx, out))
}

/// Low-level primitive used by `push_element` and `push_container`: write
/// the open tag (optionally with one attribute), run `build`, write the
/// close tag.
fn wrap_tag<T: Into<String>, R>(
    out: &mut String,
    tag: &str,
    attribute: Option<(T, T)>,
    build: impl FnOnce(&mut String) -> R,
) -> R {
    out.push('<');
    out.push_str(tag);
    if let Some((name, value)) = attribute {
        out.push(' ');
        out.push_str(&name.into());
        out.push_str("=\"");
        out.push_str(&value.into());
        out.push('"');
    }
    out.push('>');
    let result = build(out);
    out.push_str("</");
    out.push_str(tag);
    out.push_str("> ");
    result
}
