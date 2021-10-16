use std::{borrow::Borrow, collections::HashMap, fmt::Display};

use kuchiki::NodeRef;

use crate::utils::filter_names;

const SKIP_ELEMENTS: &[&str] = &[
    "button", "input", "form", "nav", "footer", "header", "script", "link", "noscript", "aside",
    "style", "head",
];
const ALLOW_OVERIDE: &[&str] = &[
    "div", "span", "section", "main", "article", "document", "body", "html", "figure",
];

const ALLOWED_ALONE: &[&str] = &["br", "hr", "img"];

/**
This enum represent an HTMLNode which can either be a Node or plain Text
Node contains three fields:
 - node name
 - attributes
 - text
*/
pub enum HTMLNode {
    Node(String, HashMap<String, String>, Vec<HTMLNode>),
    Text(String),
}

impl HTMLNode {
    pub fn from_node_ref(noderef: NodeRef) -> Option<HTMLNode> {
        if let Some((name, attrs)) = noderef
            .as_element()
            .map(|e| {
                (
                    e.name.local.to_string(),
                    e.attributes
                        .borrow()
                        .map
                        .iter()
                        .map(|(x, y)| (x.local.borrow().to_string(), y.value.to_owned()))
                        .collect(),
                )
            })
            .or_else(|| {
                noderef
                    .as_document()
                    .map(|_| ("document".to_owned(), HashMap::new()))
            })
        {
            let name = filter_names(&name);
            if SKIP_ELEMENTS.contains(&name) {
                return None;
            }
            let mut childrens = noderef
                .children()
                .flat_map(HTMLNode::from_node_ref)
                .collect::<Vec<_>>();
            if ALLOWED_ALONE.contains(&name) {
                return Some(HTMLNode::Node(name.to_owned(), attrs, childrens));
            }
            if childrens.is_empty() {
                None
            } else if ALLOW_OVERIDE.contains(&name)
                && childrens.len() == 1
                && childrens
                    .last()
                    .map(|x| matches!(x, HTMLNode::Node(..)))
                    .unwrap_or(false)
            {
                childrens.pop()
            } else {
                Some(HTMLNode::Node(name.to_owned(), attrs, childrens))
            }
        } else if let Some(e) = noderef.as_text() {
            if e.borrow().trim().is_empty() {
                None
            } else {
                Some(HTMLNode::Text(e.borrow().to_owned()))
            }
        } else {
            None
        }
    }
    pub fn get_node(&self) -> Option<&Vec<HTMLNode>> {
        match self {
            HTMLNode::Node(_, _, a) => Some(a),
            HTMLNode::Text(_) => None,
        }
    }
    pub fn get_tag_name(&self) -> Option<&str> {
        match self {
            HTMLNode::Node(a, _, _) => Some(a),
            HTMLNode::Text(_) => None,
        }
    }
    pub fn select(&self, tag_names: &[&str]) -> Vec<&Self> {
        match self {
            Self::Node(a, _, b) => {
                if tag_names.contains(&a.as_str()) {
                    vec![self]
                } else {
                    b.iter().map(|x| x.select(tag_names)).flatten().collect()
                }
            }
            Self::Text(_a) => Vec::new(),
        }
    }
}

impl Display for HTMLNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HTMLNode::Node(a, b, c) => write!(
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
                    .replace("\n", "\n  "),
                a
            ),
            HTMLNode::Text(e) => write!(f, "{}", e),
        }
    }
}
