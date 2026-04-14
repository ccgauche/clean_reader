//! Pruned HTML tree used by the text-compound lowering stage.
//!
//! This is our own tiny tree flavour built on top of a fully-parsed rcdom
//! handle. It drops structural noise (nav/footer/script/…), collapses
//! pass-through single-child wrappers (div → its child), and stores every
//! surviving element as a named-field `Element` variant.

use std::collections::HashMap;

use markup5ever_rcdom::{Handle, NodeData};

use crate::{
    error::{Error, Result},
    utils::canonical_tag,
};

/// Elements we drop unconditionally — structural noise that cannot
/// contain article content.
const BLOCKED_ELEMENTS: &[&str] = &[
    "button", "input", "form", "nav", "footer", "header", "script", "link", "noscript", "aside",
    "style", "head",
];

/// Wrapper elements that collapse into their single child when they have
/// exactly one: `<div><article>…</article></div>` becomes `<article>…</article>`.
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

/// Either a structural element or a raw text node. This is the tree fed
/// into the `TextCompound` lowering step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HTMLNode {
    Element {
        tag: String,
        attrs: HashMap<String, String>,
        children: Vec<HTMLNode>,
    },
    Text(String),
}

impl HTMLNode {
    /// Recursively concatenate the text content of this subtree.
    pub fn get_text(&self) -> String {
        fn walk(node: &HTMLNode, out: &mut String) {
            match node {
                HTMLNode::Element { children, .. } => children.iter().for_each(|c| walk(c, out)),
                HTMLNode::Text(text) => out.push_str(text),
            }
        }
        let mut out = String::new();
        walk(self, &mut out);
        out
    }

    pub fn from_handle(handle: &Handle) -> Result<HTMLNode> {
        // Text nodes short-circuit: no pruning rules apply.
        if let NodeData::Text { contents } = &handle.data {
            let text = contents.borrow();
            return if text.trim().is_empty() {
                Err(Error::EmptyText)
            } else {
                Ok(Self::Text(text.to_string()))
            };
        }

        let (raw_tag, attrs) = match &handle.data {
            NodeData::Document => (String::from("document"), HashMap::new()),
            NodeData::Element { name, attrs, .. } => (
                name.local.to_string(),
                attrs
                    .borrow()
                    .iter()
                    .map(|a| (a.name.local.to_string(), a.value.to_string()))
                    .collect::<HashMap<_, _>>(),
            ),
            _ => return Err(Error::CommentNode),
        };

        let tag = canonical_tag(&raw_tag);
        if BLOCKED_ELEMENTS.contains(&tag) {
            return Err(Error::BlockedTag {
                tag: tag.to_owned(),
            });
        }

        let mut children: Vec<HTMLNode> = handle
            .children
            .borrow()
            .iter()
            .flat_map(Self::from_handle)
            .collect();

        // Void elements are emitted as-is — even if empty.
        if VOID_ELEMENTS.contains(&tag) {
            return Ok(Self::Element {
                tag: tag.to_owned(),
                attrs,
                children,
            });
        }

        if children.is_empty() {
            return Err(Error::EmptyNode {
                tag: tag.to_owned(),
            });
        }

        // Single-child wrappers (e.g. `<div><article>…</article></div>`)
        // collapse down to their child.
        let is_wrapper_with_single_child = UNWRAP_SINGLE_CHILD.contains(&tag)
            && children.len() == 1
            && matches!(children.last(), Some(Self::Element { .. }));
        if is_wrapper_with_single_child {
            return Ok(children.pop().expect("checked len() == 1"));
        }

        Ok(Self::Element {
            tag: tag.to_owned(),
            attrs,
            children,
        })
    }

    /// Children of an `Element`, or `None` on a text node.
    pub fn children(&self) -> Option<&Vec<HTMLNode>> {
        match self {
            Self::Element { children, .. } => Some(children),
            Self::Text(_) => None,
        }
    }

    /// Tag name of an `Element`, or `None` on a text node.
    pub fn get_tag_name(&self) -> Option<&str> {
        match self {
            Self::Element { tag, .. } => Some(tag),
            Self::Text(_) => None,
        }
    }

    /// Depth-first collect all descendants whose tag is in `tag_names`.
    /// If `self` itself matches, it's returned immediately without
    /// descending into its subtree.
    pub fn select(&self, tag_names: &[&str]) -> Vec<&Self> {
        match self {
            Self::Element { tag, .. } if tag_names.contains(&tag.as_str()) => vec![self],
            Self::Element { children, .. } => {
                children.iter().flat_map(|c| c.select(tag_names)).collect()
            }
            Self::Text(_) => Vec::new(),
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
            if let HTMLNode::Element { tag, children, .. } = node {
                out.push(tag.clone());
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
