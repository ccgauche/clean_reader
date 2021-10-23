use std::borrow::Cow;

use crate::{
    html_node::HTMLNode,
    text_parser::Context,
    utils::{filter_names, get_img_link_map},
};

use super::TextCompound;

impl<'a> TextCompound<'a> {
    pub fn from_array(ctx: &mut Context<'a>, node: &'a [HTMLNode]) -> Option<Self> {
        let mut nodes: Vec<Self> = node.iter().flat_map(|x| Self::from_node(ctx, x)).collect();
        if nodes.len() <= 1 {
            nodes.pop()
        } else {
            Some(Self::Array(nodes))
        }
    }
    //Improve error handling
    pub fn from_node(ctx: &mut Context<'a>, node: &'a HTMLNode) -> Option<Self> {
        match node {
            HTMLNode::Node(a, b, c) => {
                let name = filter_names(a.as_str());
                match name {
                    "div" | "section" | "main" | "article" | "html" | "body" | "document" => {
                        Self::from_array(ctx, c)
                    }
                    "table" => Some(Self::Table(
                        node.select(&["tr"])
                            .iter()
                            .map(|x| {
                                x.select(&["td", "th"])
                                    .iter()
                                    .flat_map(|x| {
                                        Some((
                                            x.get_tag_name() == Some("th"),
                                            Self::from_array(ctx, x.get_node()?)?,
                                        ))
                                    })
                                    .collect()
                            })
                            .collect(),
                    )),
                    "p" | "time" => Some(Self::P(box Self::from_array(ctx, c)?)),
                    "a" => b
                        .get("href")
                        .map(|x| ctx.absolutize(x))
                        .map(|a| Some(Self::Link(box Self::from_array(ctx, c)?, a)))
                        .flatten()
                        .or_else(|| Self::from_array(ctx, c)),
                    "i" | "em" => Some(Self::Italic(box Self::from_array(ctx, c)?)),
                    "b" | "strong" => Some(Self::Bold(box Self::from_array(ctx, c)?)),
                    "br" | "wbr" | "hr" => Some(Self::Br),
                    "small" => Some(Self::Small(box Self::from_array(ctx, c)?)),
                    "span" | "q" => Some(Self::from_array(ctx, c)?),
                    "abbr" => Some(Self::Abbr(
                        box Self::from_array(ctx, c)?,
                        b.get("title")
                            .as_ref()
                            .map(|x| Cow::Borrowed(x.as_str()))
                            .unwrap_or_else(|| Cow::Borrowed("")),
                    )),
                    "ul" | "ol" => Some(Self::Ul(
                        c.iter()
                            .flat_map(|x| {
                                if let HTMLNode::Node(_a, _, c) = x {
                                    Self::from_array(ctx, c)
                                } else {
                                    None
                                }
                            })
                            .collect(),
                    )),
                    "sub" => Some(Self::Sub(box Self::from_array(ctx, c)?)),
                    "sup" => Some(Self::Sup(box Self::from_array(ctx, c)?)),
                    "img" => Some(Self::Img(get_img_link_map(ctx, b)?)),
                    "h1" | "h2" | "h3" | "h4" | "h5" => Some(Self::H(
                        b.get("id")
                            .map(|x| x.split(' ').map(Cow::Borrowed).collect())
                            .unwrap_or_default(),
                        name.parse().unwrap(),
                        box Self::from_array(ctx, c)?,
                    )),
                    "figure" | "figcaption" => {
                        if let Some(HTMLNode::Node(a, _, c)) = c.last() {
                            if a == "figcaption" {
                                return Some(Self::Quote(box Self::from_array(ctx, c)?));
                            }
                        }
                        Some(Self::Quote(box Self::from_array(ctx, c)?))
                    }
                    "quote" | "blockquote" => Some(Self::Quote(box Self::from_array(ctx, c)?)),
                    "cite" | "code" | "pre" => Some(Self::Code(Self::get_text(node))),
                    e => {
                        println!("Invalid element {}", e);
                        None
                    }
                }
            }
            HTMLNode::Text(e) => Some(Self::Raw(Cow::Borrowed(e))),
        }
    }
}
