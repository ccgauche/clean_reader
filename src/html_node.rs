use std::{borrow::Borrow, collections::HashMap, fmt::Display};

use kuchiki::NodeRef;

use crate::utils::filter_names;

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
                .flat_map(Self::from_node_ref)
                .collect::<Vec<_>>();
            if ALLOWED_ALONE.contains(&name) {
                Some(Self::Node(name.to_owned(), attrs, childrens))
            } else if childrens.is_empty() {
                None
            } else if ALLOW_OVERIDE.contains(&name)
                && childrens.len() == 1
                && childrens
                    .last()
                    .map(|x| matches!(x, Self::Node(..)))
                    .unwrap_or(false)
            {
                childrens.pop()
            } else {
                Some(Self::Node(name.to_owned(), attrs, childrens))
            }
        } else if let Some(e) = noderef.as_text() {
            (!e.borrow().trim().is_empty()).then(|| Self::Text(e.borrow().to_owned()))
        } else {
            None
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
