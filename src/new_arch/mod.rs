use std::{borrow::Borrow, collections::HashMap, fmt::Display};

use kuchiki::NodeRef;

use crate::{
    new_arch::{score::best_node, website_data_counter::filter_names},
    utils::gen_html_2,
};

mod score;
pub mod text_element;
mod text_parser;
mod website_data_counter;

pub enum HTMLNode {
    Node(String, HashMap<String, String>, Vec<HTMLNode>),
    Text(String),
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

const SKIP_ELEMENTS: &[&str] = &[
    "button", "input", "form", "nav", "footer", "header", "script", "link", "noscript", "aside",
    "style", "head",
];
const ALLOW_OVERIDE: &[&str] = &[
    "div", "span", "section", "main", "article", "document", "body", "html", "figure",
];

const ALLOWED_ALONE: &[&str] = &["br", "hr", "img"];

const IDS: &[&str] = &[
    // "capping",
    // "comment",
    // "related",
    // "aside",
    // "advert",
    // "inread",
    // "carousel",
    // "video",
    // "newsletter",
    // "widget",
    // "tools",
    // "login",
    // "signin",
    // "signout",
    // "sign-in",
    // "sign-out",
    // "subscribe",
    // "register",
    // "service",
    // "share",
    // "navbar",
];

const ID: &[&str] = &[/* "ads", "ad", "pub", "nav" */];

fn check_attribute(
    plurial: &[&str],
    single: &[&str],
    attribute: &str,
    attrs: &HashMap<String, String>,
) -> bool {
    if let Some(e) = attrs.get(attribute) {
        !(e.split(" ")
            .any(|x| single.contains(&x.to_lowercase().as_str()))
            || plurial.iter().any(|x| e.to_lowercase().contains(x)))
    } else {
        true
    }
}

impl HTMLNode {
    pub fn from_node_ref(noderef: NodeRef) -> Option<HTMLNode> {
        if let Some((name, attrs)) = noderef
            .as_element()
            .map(|e| {
                (
                    e.name.local.borrow().to_string(),
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
            if !check_attribute(IDS, ID, "id", &attrs) || !check_attribute(IDS, ID, "class", &attrs)
            {
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
}

pub fn run_v2(url: &str) -> anyhow::Result<String> {
    use kuchiki::traits::TendrilSink;
    let document = kuchiki::parse_html().one(crate::utils::http_get(url)?);
    let h = crate::title_extractor::try_extract_data(&document);
    let html = HTMLNode::from_node_ref(document)
        .ok_or_else(|| anyhow::anyhow!("Invalid HTMLNode ref generation"))?;
    let ctx = crate::text_parser::Context {
        meta: h,
        url: reqwest::Url::parse(url).expect("Invalid URL"),
    };
    //println!("{}", html);
    let text = text_element::TextCompound::from_html_node(&ctx, best_node(&html))
        .ok_or_else(|| anyhow::anyhow!("Invalid HTML generation"))?;
    //println!("{}", text_element::Compilable::html(&text).unwrap());
    Ok(gen_html_2(&[text], &ctx))
}
