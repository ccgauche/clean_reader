//! DOM-to-`TextCompound` lowering. Consumes a pruned [`HTMLNode`] tree and
//! produces the rich-text IR used by the HTML/template stages.

use std::borrow::Cow;

use crate::{
    context::Context,
    html_node::HTMLNode,
    utils::{canonical_tag, extract_image_src},
};

use super::{Row, Table, TableCell, TextCompound};

impl<'a> TextCompound<'a> {
    /// Flatten `self` into plain text. Used for heading dedup against the
    /// page title.
    pub fn text(&'a self) -> Cow<'a, str> {
        match self {
            Self::Raw(text) => Cow::Borrowed(text),
            Self::Code(text) => Cow::Borrowed(text),
            Self::Link { content, .. }
            | Self::Abbr { content, .. }
            | Self::Heading { content, .. } => content.text(),
            Self::Italic(child)
            | Self::Bold(child)
            | Self::Sup(child)
            | Self::Sub(child)
            | Self::Underline(child)
            | Self::Small(child)
            | Self::P(child)
            | Self::Quote(child) => child.text(),
            Self::Array(items) | Self::Ul(items) => {
                Cow::Owned(items.iter().map(|item| item.text()).collect::<String>())
            }
            Self::Img(_) | Self::Br => Cow::Borrowed(""),
            Self::Table(table) => Cow::Owned(
                table
                    .rows
                    .iter()
                    .flat_map(|row| row.cells.iter().map(|cell| cell.content().text()))
                    .collect::<String>(),
            ),
        }
    }

    /// Lower a slice of sibling nodes into one `TextCompound`. A single
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
            HTMLNode::Element {
                tag,
                attrs,
                children,
            } => (tag, attrs, children),
            HTMLNode::Text(text) => return Some(Self::raw(text.as_str())),
        };

        match canonical_tag(tag.as_str()) {
            "div" | "section" | "main" | "article" | "html" | "body" | "document" => {
                Self::from_array(ctx, children)
            }
            "table" => Some(Self::Table(lower_table(ctx, node))),
            "time" => Self::from_array(ctx, children).map(Self::paragraph),
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
                    Self::from_array(ctx, children).map(Self::paragraph)
                }
            }
            "a" => match attrs.get("href") {
                Some(raw) => {
                    let href = ctx.absolutize(raw);
                    Self::from_array(ctx, children).map(|body| Self::link(body, href))
                }
                None => Self::from_array(ctx, children),
            },
            "u" => Self::from_array(ctx, children).map(Self::underline),
            "i" | "em" => Self::from_array(ctx, children).map(Self::italic),
            "b" | "strong" => Self::from_array(ctx, children).map(Self::bold),
            "br" | "wbr" | "hr" => Some(Self::Br),
            "small" => Self::from_array(ctx, children).map(Self::small),
            "span" | "q" => Self::from_array(ctx, children),
            "abbr" => {
                let title = attrs.get("title").map(String::as_str).unwrap_or("");
                Self::from_array(ctx, children).map(|body| Self::abbr(body, title))
            }
            "ul" | "ol" => Some(Self::Ul(
                children
                    .iter()
                    .filter_map(|item| Self::from_array(ctx, item.children()?))
                    .collect(),
            )),
            "sub" => Self::from_array(ctx, children).map(Self::sub),
            "sup" => Self::from_array(ctx, children).map(Self::sup),
            "img" => extract_image_src(ctx, attrs).map(Self::img),
            heading_tag @ ("h1" | "h2" | "h3" | "h4" | "h5") => {
                let body = Self::from_array(ctx, children)?;
                // Drop a heading whose text matches the page title — we
                // don't want to render the title twice.
                if let Some(page_title) = &ctx.meta.title {
                    if alphanumeric_eq(page_title, &body.text()) {
                        return None;
                    }
                }
                let fragment_ids: Vec<&str> = attrs
                    .get("id")
                    .map(|v| v.split(' ').collect())
                    .unwrap_or_default();
                Some(Self::heading(heading_tag.parse().ok()?, fragment_ids, body))
            }
            "figure" | "figcaption" => {
                // Prefer the `<figcaption>` child if one is present as the
                // last element; otherwise fall back to the figure body.
                let inner = if let Some(HTMLNode::Element {
                    tag: last_tag,
                    children: caption_children,
                    ..
                }) = children.last()
                {
                    if last_tag == "figcaption" {
                        Self::from_array(ctx, caption_children)?
                    } else {
                        Self::from_array(ctx, children)?
                    }
                } else {
                    Self::from_array(ctx, children)?
                };
                Some(Self::quote(inner))
            }
            "quote" | "blockquote" => Self::from_array(ctx, children).map(Self::quote),
            "cite" | "code" | "pre" => Some(Self::Code(node.get_text())),
            "math" => None, // not supported yet
            unknown => {
                eprintln!("unsupported element <{}>", unknown);
                None
            }
        }
    }
}

/// Lower a `<table>` into the `Table { rows: Vec<Row { cells: … }> }`
/// hierarchy. Rows with no lowered cells are kept as empty rows so the
/// grid retains its shape.
fn lower_table<'a>(ctx: &mut Context<'a>, table_node: &'a HTMLNode) -> Table<'a> {
    let rows = table_node
        .select(&["tr"])
        .iter()
        .map(|row_node| Row {
            cells: row_node
                .select(&["td", "th"])
                .iter()
                .filter_map(|cell_node| lower_cell(ctx, cell_node))
                .collect(),
        })
        .collect();
    Table { rows }
}

fn lower_cell<'a>(ctx: &mut Context<'a>, cell_node: &'a HTMLNode) -> Option<TableCell<'a>> {
    let content = TextCompound::from_array(ctx, cell_node.children()?)?;
    Some(match cell_node.get_tag_name() {
        Some("th") => TableCell::Header(content),
        _ => TableCell::Data(content),
    })
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
