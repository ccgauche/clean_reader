//! Pruned HTML tree used by the text-compound lowering stage.
//!
//! This is our own tiny tree flavor built on top of a fully-parsed rcdom
//! handle. It drops structural noise (nav/footer/script/…), collapses
//! pass-through single-child wrappers (div → its child), and stores every
//! surviving element as `(tag, attrs, children)`.

use std::collections::HashMap;

use markup5ever_rcdom::{Handle, NodeData};

use crate::{
    error::{Error, Result},
    utils::canonical_tag,
};

/// Elements we drop unconditionally — structural noise that cannot contain
/// article content.
const BLOCKED_ELEMENTS: &[&str] = &[
    "button", "input", "form", "nav", "footer", "header", "script", "link", "noscript", "aside",
    "style", "head",
];

/// Wrapper elements that should collapse into their single child when they
/// have exactly one: `<div><article>…</article></div>` becomes `<article>…</article>`.
const UNWRAP_SINGLE_CHILD: &[&str] = &[
    "div",
    "span",
    "section",
    "main",
    "article",
    "document",
    "body",
    "html",
    "figure",
    "amp-script",
];

/// Elements allowed to exist without any children (void elements and images).
const VOID_ELEMENTS: &[&str] = &["br", "hr", "img"];

/// Either a structural element (`tag`, `attrs`, `children`) or a raw text
/// node. This is the tree fed into the `TextCompound` lowering step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HTMLNode {
    Node(String, HashMap<String, String>, Vec<HTMLNode>),
    Text(String),
}

impl HTMLNode {
    pub fn get_text(&self) -> String {
        fn walk(node: &HTMLNode, out: &mut String) {
            match node {
                HTMLNode::Node(_, _, c) => c.iter().for_each(|x| walk(x, out)),
                HTMLNode::Text(a) => out.push_str(a),
            }
        }
        let mut s = String::new();
        walk(self, &mut s);
        s
    }

    pub fn from_handle(handle: &Handle) -> Result<HTMLNode> {
        let (name, attrs): (String, HashMap<String, String>) = match &handle.data {
            NodeData::Document => ("document".to_owned(), HashMap::new()),
            NodeData::Element { name, attrs, .. } => (
                name.local.to_string(),
                attrs
                    .borrow()
                    .iter()
                    .map(|a| (a.name.local.to_string(), a.value.to_string()))
                    .collect(),
            ),
            NodeData::Text { contents } => {
                let text = contents.borrow();
                return if text.trim().is_empty() {
                    Err(Error::EmptyText)
                } else {
                    Ok(Self::Text(text.to_string()))
                };
            }
            NodeData::Comment { .. }
            | NodeData::Doctype { .. }
            | NodeData::ProcessingInstruction { .. } => return Err(Error::CommentNode),
        };

        let name = canonical_tag(&name);
        if BLOCKED_ELEMENTS.contains(&name) {
            return Err(Error::BlockedTag {
                tag: name.to_owned(),
            });
        }
        let mut children = handle
            .children
            .borrow()
            .iter()
            .flat_map(Self::from_handle)
            .collect::<Vec<_>>();
        if VOID_ELEMENTS.contains(&name) {
            Ok(Self::Node(name.to_owned(), attrs, children))
        } else if children.is_empty() {
            Err(Error::EmptyNode {
                tag: name.to_owned(),
            })
        } else if UNWRAP_SINGLE_CHILD.contains(&name)
            && children.len() == 1
            && children
                .last()
                .map(|x| matches!(x, Self::Node(..)))
                .unwrap_or(false)
        {
            children.pop().ok_or_else(|| Error::EmptyNode {
                tag: name.to_owned(),
            })
        } else {
            Ok(Self::Node(name.to_owned(), attrs, children))
        }
    }

    pub fn get_node(&self) -> Option<&Vec<HTMLNode>> {
        if let Self::Node(_, _, a) = self {
            Some(a)
        } else {
            None
        }
    }

    pub fn get_tag_name(&self) -> Option<&str> {
        if let Self::Node(a, _, _) = self {
            Some(a)
        } else {
            None
        }
    }

    pub fn select(&self, tag_names: &[&str]) -> Vec<&Self> {
        if let Self::Node(a, _, b) = self {
            if tag_names.contains(&a.as_str()) {
                vec![self]
            } else {
                b.iter().flat_map(|x| x.select(tag_names)).collect()
            }
        } else {
            Vec::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use html5ever::tendril::TendrilSink;

    fn parse(html: &str) -> HTMLNode {
        let dom = html5ever::parse_document(
            markup5ever_rcdom::RcDom::default(),
            html5ever::ParseOpts::default(),
        )
        .one(html);
        HTMLNode::from_handle(&dom.document).expect("parse")
    }

    fn tags(node: &HTMLNode) -> Vec<String> {
        let mut out = Vec::new();
        fn walk(node: &HTMLNode, out: &mut Vec<String>) {
            if let HTMLNode::Node(name, _, children) = node {
                out.push(name.clone());
                for c in children {
                    walk(c, out);
                }
            }
        }
        walk(node, &mut out);
        out
    }

    #[test]
    fn strips_blocked_tags() {
        let node = parse("<html><body><p>hi</p><script>alert(1)</script></body></html>");
        assert!(!tags(&node).iter().any(|t| t == "script"));
    }

    #[test]
    fn empty_text_is_dropped() {
        // A whitespace-only <p> should prune away; adjacent real content survives.
        let node = parse("<html><body><p>   </p><p>real</p></body></html>");
        let ts = tags(&node);
        assert_eq!(ts.iter().filter(|t| t.as_str() == "p").count(), 1);
    }

    #[test]
    fn preserves_heading_and_link() {
        let node = parse("<html><body><h1>Title</h1><a href=\"/x\">link</a></body></html>");
        let ts = tags(&node);
        assert!(ts.contains(&"h1".to_string()));
        assert!(ts.contains(&"a".to_string()));
    }
}
