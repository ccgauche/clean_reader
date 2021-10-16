use std::borrow::Cow;

use crate::{text_parser::Context, utils::get_img_link_map};

use super::{
    filter_names,
    html_node::HTMLNode,
    text_element::{Header, TextCompound},
};

impl<'a> TextCompound<'a> {
    pub fn get_text(node: &HTMLNode) -> String {
        fn inner(node: &HTMLNode, string: &mut String) {
            match node {
                HTMLNode::Node(_, _, c) => c.iter().for_each(|x| inner(x, string)),
                HTMLNode::Text(a) => string.push_str(a),
            }
        }
        let mut s = String::new();
        inner(node, &mut s);
        s
    }
    pub fn from_html_node_array(ctx: &mut Context<'a>, node: &'a [HTMLNode]) -> Option<Self> {
        let mut nodes: Vec<Self> = node
            .iter()
            .flat_map(|x| Self::from_html_node(ctx, x))
            .collect();
        if nodes.is_empty() {
            None
        } else if nodes.len() == 1 {
            nodes.pop()
        } else {
            Some(Self::Array(nodes))
        }
    }
    pub fn from_html_node(ctx: &mut Context<'a>, node: &'a HTMLNode) -> Option<Self> {
        match node {
            HTMLNode::Node(a, b, c) => match filter_names(a.as_str()) {
                "div" | "section" | "main" | "article" | "html" | "body" => {
                    Self::from_html_node_array(ctx, c)
                }
                "table" => Some(Self::Table(
                    node.select(&["tr"])
                        .iter()
                        .map(|x| {
                            x.select(&["td", "th"])
                                .iter()
                                .map(|x| {
                                    Some((
                                        x.get_tag_name() == Some("th"),
                                        Self::from_html_node_array(ctx, x.get_node().unwrap())?,
                                    ))
                                })
                                .flatten()
                                .collect()
                        })
                        .collect(),
                )),
                "p" | "time" => Some(TextCompound::P(box Self::from_html_node_array(ctx, c)?)),
                "a" => {
                    let html = Self::from_html_node_array(ctx, c)?;
                    let k = b
                        .get("href")
                        //.filter(|x| !x.starts_with("#"))
                        .map(|x| ctx.absolutize(x));
                    if let Some(a) = k {
                        Some(TextCompound::Link(box html, a))
                    } else {
                        Some(html)
                    }
                }
                "i" | "em" => Some(TextCompound::Italic(box Self::from_html_node_array(
                    ctx, c,
                )?)),
                "b" | "strong" => Some(TextCompound::Bold(box Self::from_html_node_array(ctx, c)?)),
                "br" | "wbr" | "hr" => Some(TextCompound::Br),
                "small" => Some(TextCompound::Small(box Self::from_html_node_array(ctx, c)?)),
                "span" | "q" => Some(Self::from_html_node_array(ctx, c)?),
                "abbr" => Some(TextCompound::Abbr(
                    box Self::from_html_node_array(ctx, c)?,
                    b.get("title")
                        .as_ref()
                        .map(|x| Cow::Borrowed(x.as_str()))
                        .unwrap_or_else(|| Cow::Borrowed("")),
                )),
                "ul" | "ol" => Some(TextCompound::Ul(
                    c.iter()
                        .flat_map(|x| {
                            if let HTMLNode::Node(_a, _, c) = x {
                                Self::from_html_node_array(ctx, c)
                            } else {
                                None
                            }
                        })
                        .collect(),
                )),
                "document" => Self::from_html_node_array(ctx, c),
                "sub" => Some(TextCompound::Sub(box Self::from_html_node_array(ctx, c)?)),
                "sup" => Some(TextCompound::Sup(box Self::from_html_node_array(ctx, c)?)),
                "img" => Some(TextCompound::Img(get_img_link_map(ctx, b)?)),
                // TODO(code style): Transform all of this into one statement
                "h1" => Some(TextCompound::H(
                    b.get("id")
                        .map(|x| x.split(' ').map(Cow::Borrowed).collect())
                        .unwrap_or_default(),
                    Header::H1,
                    box Self::from_html_node_array(ctx, c)?,
                )),
                "h2" => Some(TextCompound::H(
                    b.get("id")
                        .map(|x| x.split(' ').map(Cow::Borrowed).collect())
                        .unwrap_or_default(),
                    Header::H2,
                    box Self::from_html_node_array(ctx, c)?,
                )),
                "h3" => Some(TextCompound::H(
                    b.get("id")
                        .map(|x| x.split(' ').map(Cow::Borrowed).collect())
                        .unwrap_or_default(),
                    Header::H3,
                    box Self::from_html_node_array(ctx, c)?,
                )),
                "h4" => Some(TextCompound::H(
                    b.get("id")
                        .map(|x| x.split(' ').map(Cow::Borrowed).collect())
                        .unwrap_or_default(),
                    Header::H4,
                    box Self::from_html_node_array(ctx, c)?,
                )),
                "h5" => Some(TextCompound::H(
                    b.get("id")
                        .map(|x| x.split(' ').map(Cow::Borrowed).collect())
                        .unwrap_or_default(),
                    Header::H5,
                    box Self::from_html_node_array(ctx, c)?,
                )),
                "figure" | "figcaption" => {
                    if let Some(HTMLNode::Node(a, _, c)) = c.last() {
                        if a == "figcaption" {
                            return Some(TextCompound::Quote(box Self::from_html_node_array(
                                ctx, c,
                            )?));
                        }
                    }
                    Some(TextCompound::Quote(box TextCompound::from_html_node_array(
                        ctx, c,
                    )?))
                }
                "quote" | "blockquote" => {
                    Some(TextCompound::Quote(box Self::from_html_node_array(ctx, c)?))
                }
                "cite" | "code" | "pre" => Some(TextCompound::Code(Self::get_text(node))),
                e => {
                    println!("Invalid element {}", e);
                    None
                }
            },
            HTMLNode::Text(e) => Some(Self::Raw(Cow::Borrowed(e))),
        }
    }
}
