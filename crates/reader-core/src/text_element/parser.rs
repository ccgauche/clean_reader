//! DOM-to-`TextCompound` lowering. Consumes a pruned [`HTMLNode`] tree and
//! produces the rich-text IR used by the HTML/template stages.

use std::borrow::Cow;

use crate::{
    html_node::HTMLNode,
    text_parser::Context,
    utils::{canonical_tag, extract_image_src},
};

use super::TextCompound;

impl<'a> TextCompound<'a> {
    /// Flatten `self` into plain text. Used for heading dedup against the
    /// page title and for the markdown export (when that existed).
    pub fn text(&'a self) -> Cow<'a, str> {
        match self {
            Self::Raw(a) => Cow::Borrowed(a),
            Self::Code(a) => Cow::Borrowed(a),
            Self::Link(child, _) => child.text(),
            Self::Italic(child)
            | Self::Bold(child)
            | Self::Sup(child)
            | Self::Sub(child)
            | Self::Underline(child)
            | Self::Small(child)
            | Self::Abbr(child, _)
            | Self::P(child)
            | Self::Quote(child)
            | Self::H(_, _, child) => child.text(),
            Self::Array(items) | Self::Ul(items) => {
                Cow::Owned(items.iter().map(|item| item.text()).collect::<String>())
            }
            Self::Img(_) | Self::Br => Cow::Borrowed(""),
            Self::Table(rows) => Cow::Owned(
                rows.iter()
                    .flat_map(|row| row.iter().map(|(_, cell)| cell.text()))
                    .collect::<String>(),
            ),
        }
    }

    /// Lower a slice of sibling nodes into one `TextCompound`. Single
    /// survivor is returned directly; multiple survivors get wrapped in
    /// [`TextCompound::Array`].
    pub fn from_array(ctx: &mut Context<'a>, siblings: &'a [HTMLNode]) -> Option<Self> {
        let mut lowered: Vec<Self> = siblings
            .iter()
            .flat_map(|child| Self::from_node(ctx, child))
            .collect();
        if lowered.len() <= 1 {
            lowered.pop()
        } else {
            Some(Self::Array(lowered))
        }
    }

    pub fn from_node(ctx: &mut Context<'a>, node: &'a HTMLNode) -> Option<Self> {
        let (tag, attrs, children) = match node {
            HTMLNode::Node(tag, attrs, children) => (tag, attrs, children),
            HTMLNode::Text(text) => return Some(Self::Raw(Cow::Borrowed(text))),
        };

        match canonical_tag(tag.as_str()) {
            "div" | "section" | "main" | "article" | "html" | "body" | "document" => {
                Self::from_array(ctx, children)
            }
            "table" => Some(Self::Table(lower_table(ctx, node))),
            "time" => Some(Self::P(Box::new(Self::from_array(ctx, children)?))),
            "p" => {
                let is_code_block = attrs
                    .get("class")
                    .map(|class| class.contains("code"))
                    .unwrap_or(false);
                if is_code_block {
                    let mut text = node.get_text();
                    text.push('\n');
                    Some(Self::Code(text))
                } else {
                    Some(Self::P(Box::new(Self::from_array(ctx, children)?)))
                }
            }
            "a" => attrs
                .get("href")
                .map(|raw| ctx.absolutize(raw))
                .and_then(|href| Some(Self::Link(Box::new(Self::from_array(ctx, children)?), href)))
                .or_else(|| Self::from_array(ctx, children)),
            "u" => Some(Self::Underline(Box::new(Self::from_array(ctx, children)?))),
            "i" | "em" => Some(Self::Italic(Box::new(Self::from_array(ctx, children)?))),
            "b" | "strong" => Some(Self::Bold(Box::new(Self::from_array(ctx, children)?))),
            "br" | "wbr" | "hr" => Some(Self::Br),
            "small" => Some(Self::Small(Box::new(Self::from_array(ctx, children)?))),
            "span" | "q" => Self::from_array(ctx, children),
            "abbr" => {
                let title = attrs
                    .get("title")
                    .map(|t| Cow::Borrowed(t.as_str()))
                    .unwrap_or_else(|| Cow::Borrowed(""));
                Some(Self::Abbr(
                    Box::new(Self::from_array(ctx, children)?),
                    title,
                ))
            }
            "ul" | "ol" => Some(Self::Ul(
                children
                    .iter()
                    .filter_map(|item| Self::from_array(ctx, item.get_node()?))
                    .collect(),
            )),
            "sub" => Some(Self::Sub(Box::new(Self::from_array(ctx, children)?))),
            "sup" => Some(Self::Sup(Box::new(Self::from_array(ctx, children)?))),
            "img" => Some(Self::Img(extract_image_src(ctx, attrs)?)),
            tag @ ("h1" | "h2" | "h3" | "h4" | "h5") => {
                let heading_body = Self::from_array(ctx, children)?;
                // Drop a heading whose text matches the page title — we
                // don't want to render the title twice.
                if let Some(page_title) = &ctx.meta.title {
                    if alphanumeric_eq(page_title, &heading_body.text()) {
                        return None;
                    }
                }
                let ids = attrs
                    .get("id")
                    .map(|v| v.split(' ').map(Cow::Borrowed).collect())
                    .unwrap_or_default();
                Some(Self::H(ids, tag.parse().ok()?, Box::new(heading_body)))
            }
            "figure" | "figcaption" => {
                // Prefer the `<figcaption>` child if one is present as the
                // last element; otherwise fall back to the figure body.
                let inner =
                    if let Some(HTMLNode::Node(last_tag, _, caption_children)) = children.last() {
                        if last_tag == "figcaption" {
                            Self::from_array(ctx, caption_children)?
                        } else {
                            Self::from_array(ctx, children)?
                        }
                    } else {
                        Self::from_array(ctx, children)?
                    };
                Some(Self::Quote(Box::new(inner)))
            }
            "quote" | "blockquote" => Some(Self::Quote(Box::new(Self::from_array(ctx, children)?))),
            "cite" | "code" | "pre" => Some(Self::Code(node.get_text())),
            "math" => None, // not supported yet
            unknown => {
                eprintln!("unsupported element <{}>", unknown);
                None
            }
        }
    }
}

/// Lower a `<table>` into our row-of-cells IR.
fn lower_table<'a>(
    ctx: &mut Context<'a>,
    table: &'a HTMLNode,
) -> Vec<Vec<(bool, TextCompound<'a>)>> {
    table
        .select(&["tr"])
        .iter()
        .map(|row| {
            row.select(&["td", "th"])
                .iter()
                .filter_map(|cell| {
                    let is_header = cell.get_tag_name() == Some("th");
                    Some((is_header, TextCompound::from_array(ctx, cell.get_node()?)?))
                })
                .collect()
        })
        .collect()
}

/// Whether two strings are equal after dropping all non-ASCII-alphanumeric
/// characters. Used to dedup a heading against the page title without
/// caring about punctuation or whitespace differences.
fn alphanumeric_eq(a: &str, b: &str) -> bool {
    fn alnum(s: &str) -> impl Iterator<Item = char> + '_ {
        s.chars().filter(char::is_ascii_alphanumeric)
    }
    alnum(a).eq(alnum(b))
}
