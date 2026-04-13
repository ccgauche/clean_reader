use std::{collections::HashMap, fmt::Display};

use markup5ever_rcdom::{Handle, NodeData};

use crate::{
    error::{Error, Result},
    utils::filter_names,
};

const SKIP_ELEMENTS: &[&str] = &[
    "button", "input", "form", "nav", "footer", "header", "script", "link", "noscript", "aside",
    "style", "head",
];
const ALLOW_OVERIDE: &[&str] = &[
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

const ALLOWED_ALONE: &[&str] = &["br", "hr", "img"];

/**
This enum represent an HTMLNode which can either be a Node or plain Text
Node contains three fields:
 - node name
 - attributes
 - text
*/
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HTMLNode {
    Node(String, HashMap<String, String>, Vec<HTMLNode>),
    Text(String),
}

impl HTMLNode {
    pub fn get_text(&self) -> String {
        fn inner(node: &HTMLNode, string: &mut String) {
            match node {
                HTMLNode::Node(_, _, c) => c.iter().for_each(|x| inner(x, string)),
                HTMLNode::Text(a) => string.push_str(a),
            }
        }
        let mut s = String::new();
        inner(self, &mut s);
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

        let name = filter_names(&name);
        if SKIP_ELEMENTS.contains(&name) {
            return Err(Error::BlockedTag {
                tag: name.to_owned(),
            });
        }
        let mut childrens = handle
            .children
            .borrow()
            .iter()
            .flat_map(Self::from_handle)
            .collect::<Vec<_>>();
        if ALLOWED_ALONE.contains(&name) {
            Ok(Self::Node(name.to_owned(), attrs, childrens))
        } else if childrens.is_empty() {
            Err(Error::EmptyNode {
                tag: name.to_owned(),
            })
        } else if ALLOW_OVERIDE.contains(&name)
            && childrens.len() == 1
            && childrens
                .last()
                .map(|x| matches!(x, Self::Node(..)))
                .unwrap_or(false)
        {
            childrens.pop().ok_or_else(|| Error::EmptyNode {
                tag: name.to_owned(),
            })
        } else {
            Ok(Self::Node(name.to_owned(), attrs, childrens))
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

impl HTMLNode {
    #[allow(dead_code)]
    pub fn display(&self) -> String {
        match self {
            Self::Node(a, _, c) if c.is_empty() => format!("</{}>", a),
            Self::Node(a, _, c) => format!(
                "<{}>\n  {}\n</{}>",
                a,
                c.iter()
                    .map(|x| x.display())
                    .collect::<Vec<_>>()
                    .join("\n")
                    .replace('\n', "\n  "),
                a
            ),
            Self::Text(e) => e.to_string(),
        }
    }
}

impl Display for HTMLNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Node(a, b, c) => write!(
                f,
                "<{} {}>\n  {}\n</{}>",
                a,
                b.iter()
                    .map(|x| format!("{}={:?}", x.0, x.1))
                    .collect::<Vec<_>>()
                    .join(" "),
                c.iter()
                    .map(|x| x.to_string())
                    .collect::<Vec<_>>()
                    .join("\n")
                    .replace('\n', "\n  "),
                a
            ),
            Self::Text(e) => write!(f, "{}", e),
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
        // <script> is in SKIP_ELEMENTS — should never appear in the result tree.
        let node = parse("<html><body><p>hi</p><script>alert(1)</script></body></html>");
        assert!(!tags(&node).iter().any(|t| t == "script"));
    }

    #[test]
    fn empty_text_is_dropped() {
        // A whitespace-only `<p>` should prune away, but adjacent real content survives.
        let node = parse("<html><body><p>   </p><p>real</p></body></html>");
        let ts = tags(&node);
        // Exactly one <p> should remain — the one with real text.
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
