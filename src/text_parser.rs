use std::{
    borrow::{Borrow, Cow},
    collections::HashMap,
    hash::Hash,
    ops::AddAssign,
};

use kuchiki::NodeRef;
use reqwest::Url;

use crate::{
    structures::{Header, Part, TextCompound},
    synthax::code_from_div,
    title_extractor::ArticleData,
    utils::{get_img_link, valid_text},
};

/**
All the element that won't get any computing (Automaticaly deletted when seen)
*/
const ELEMENTS_TO_IGNORE: &[&str] = &["script", "link", "style", "nav", "footer", "header"];

/**
This function converts the raw html tree to the IR.
 */
pub fn clean_html(anchor: &NodeRef, ctx: &Context) -> Vec<Part<'static>> {
    let mut parts = Vec::new();
    for i in anchor.children() {
        extract_text_image_parts(ctx, &i, &mut parts);
    }
    let mut parts1 = Vec::new();
    for k in find_main_content(&parts).children() {
        parse(ctx, &k, &mut parts1);
    }
    parts1
}

/**
This generic function insert `one` at key if it doesn't exists or add `one` to the existing value.
*/
fn insert_or_increment<T: Eq + Hash, E: AddAssign + Default + Clone>(
    map: &mut HashMap<T, E>,
    key: T,
    one: E,
) {
    if let Some(a) = map.get_mut(&key) {
        *a += one;
    } else {
        map.insert(key, one);
    }
}

/**
This wrapper enables the use of NodeRef in an HashMap providing the Hash implemeentation
*/
#[derive(PartialEq, Eq, Clone, Debug)]
struct NodeRefWrapper(NodeRef);

impl Hash for NodeRefWrapper {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        if let Some(e) = self.0.as_text() {
            state.write(e.borrow().as_bytes())
        } else if self.0.as_document().is_some() {
            state.write_i8(2);
        } else if self.0.as_doctype().is_some() {
            state.write_i8(3);
        } else if let Some(e) = self.0.as_element() {
            state.write(format!("{:?}", e).as_bytes());
        } else {
            state.write_i8(4);
        }
    }
}

/**
Scoring map:
This defines How much points gives each element.
a and h2 is 0 because they are often used in navs and related articles footers.
*/
const EMAP: &[(&str, u32)] = &[("p", 4), ("a", 0), ("h2", 0)];

/**
This functions finds where the article body is using text repartition on the webpage and element scoring.
Will stop searching after the third parent of a absolute text element (p, pre...)
*/
pub fn find_main_content(vec: &[NodeRef]) -> NodeRef {
    let mut map1: HashMap<NodeRefWrapper, u32> = HashMap::new();
    let mut map2: HashMap<NodeRefWrapper, u32> = HashMap::new();
    let mut map3: HashMap<NodeRefWrapper, u32> = HashMap::new();

    for i in vec {
        let k = if let Some(e) = i.as_element() {
            let local = e.name.local.borrow().to_string();
            EMAP.iter()
                .find(|(x, _)| *x == local)
                .map(|(_, x)| *x)
                .unwrap_or(1)
        } else {
            1
        };
        let parent = i.parent().unwrap();
        insert_or_increment(&mut map1, NodeRefWrapper(parent.clone()), k);
        let parent = parent.parent().unwrap();
        insert_or_increment(&mut map2, NodeRefWrapper(parent.clone()), k);
        let parent = parent.parent().unwrap();
        insert_or_increment(&mut map3, NodeRefWrapper(parent.clone()), k);
    }
    let (mut a, mut b) = find_best(map1);
    let (c, d) = find_best(map2);
    if (d as f32 * 0.8) as u32 > b {
        b = d;
        a = c;
    }
    let (c, d) = find_best(map3);
    if (d as f32 * 0.6) as u32 > b {
        a = c;
    }
    a
}

/*
This function extracts the NodeRefWrapper with the biggest score from the map
*/
fn find_best(map1: HashMap<NodeRefWrapper, u32>) -> (NodeRef, u32) {
    let mut map1 = map1.into_iter();
    let mut refe = map1.next().map(|o| (o.0 .0, o.1)).unwrap();
    for o in map1 {
        if o.1 > refe.1 {
            refe = (o.0 .0, o.1);
        }
    }
    refe
}

/**
List of elements that will not be checked for text content since they always contains text
*/
const TEXT_ONLY_ELEMENTS: &[&str] = &[
    "a",
    "i",
    "s",
    "strong",
    "italic",
    "span",
    "p",
    "pre",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "em",
    "abbr",
    "quote",
    "sub",
    "sup",
    "cite",
    "blockquote",
    "code",
    "q",
];

/**
This function is used to debug the html (This displays legacy html from NodeRef)
*/
#[allow(unused)]
pub fn display_html(tabs: usize, node: &NodeRef) {
    if let Some(e) = node.as_element() {
        println!(
            "{}<{} {}>",
            (0..tabs).map(|_| [' ', ' ']).flatten().collect::<String>(),
            e.name.local.to_string(),
            e.attributes
                .borrow()
                .map
                .iter()
                .map(|(x, y)| { format!("{}={:?}", x.local.to_string(), y.value.to_string()) })
                .collect::<Vec<String>>()
                .join(" ")
        );
        for i in node.children() {
            display_html(tabs + 1, &i);
        }
        println!(
            "{}</{}>",
            (0..tabs).map(|_| [' ', ' ']).flatten().collect::<String>(),
            e.name.local.to_string()
        );
    } else if let Some(e) = node.as_text() {
        println!(
            "{}{:?}",
            (0..tabs).map(|_| [' ', ' ']).flatten().collect::<String>(),
            e.borrow()
        );
    }
}

/**
This function will extract image and texts from the html so we can search for the main content using repartition and scoring.

*/
pub fn extract_text_image_parts(ctx: &Context, node: &NodeRef, parts: &mut Vec<NodeRef>) {
    if let Some(e) = node.as_element() {
        let local = e.name.local.to_string();
        if ELEMENTS_TO_IGNORE.iter().any(|x| x == &local) {
            return;
        }
        let k = local == "div"
            && e.attributes
                .borrow()
                .get("class")
                .map(|x| x.contains("highlight"))
                .unwrap_or(false);
        if k || (valid_text(&node.text_contents(), ctx, &local)
            || node.text_contents().trim().is_empty())
        {
            let to_find = node.parent().unwrap();
            parts.retain(|x| x != &to_find);
            parts.push(node.clone());
            if !k && !TEXT_ONLY_ELEMENTS.iter().any(|x| x == &local) {
                node.children()
                    .for_each(|x| extract_text_image_parts(ctx, &x, parts));
            }
        }
    }
}

/**
The context of the parser (The current url for link absolutization and the article data to avoid including multiple time the same title)
*/
pub struct Context {
    pub url: Url,
    pub meta: ArticleData,
}

/**
Parse lambda HTML to parts.
*/
pub fn parse(ctx: &Context, node: &NodeRef, parts: &mut Vec<Part<'static>>) {
    if let Some(e) = node.as_element() {
        let local = e.name.local.to_string();
        for k in ELEMENTS_TO_IGNORE {
            if &local == k {
                return;
            }
        }
        if !(valid_text(&node.text_contents(), ctx, &local)
            || node.text_contents().trim().is_empty() && contains_img(node))
        {
            return;
        }
        if e.attributes.borrow().contains("data-move-to")
            || e.attributes.borrow().contains("hidden")
        {
            return;
        }
        if local == "h1" {
            let k = Part::H(
                Header::H1,
                TextCompound::Array({
                    let i = node
                        .children()
                        .flat_map(|x| to_text(ctx, &x, false, parts))
                        .collect::<Vec<_>>();
                    if i.is_empty() {
                        return;
                    }
                    i
                }),
            );
            parts.push(k)
        } else if local == "code"
            || local == "pre"
            || (local == "div"
                && e.attributes
                    .borrow()
                    .get("class")
                    .map(|x| x.contains("highlight"))
                    .unwrap_or(false))
        {
            parts.push(Part::PlainText(TextCompound::Code(Cow::Owned(
                code_from_div(node),
            ))));
        } else if local == "h2" {
            let o = Part::H(
                Header::H2,
                TextCompound::Array({
                    let i = node
                        .children()
                        .flat_map(|x| to_text(ctx, &x, false, parts))
                        .collect::<Vec<_>>();
                    if i.is_empty() {
                        return;
                    }
                    i
                }),
            );
            parts.push(o)
        } else if local == "h3" {
            let k = Part::H(
                Header::H3,
                TextCompound::Array({
                    let i = node
                        .children()
                        .flat_map(|x| to_text(ctx, &x, false, parts))
                        .collect::<Vec<_>>();
                    if i.is_empty() {
                        return;
                    }
                    i
                }),
            );
            parts.push(k)
        } else if local == "quote" || local == "blockquote" || local == "cite" {
            let k = Part::Quote(TextCompound::Array({
                let i = node
                    .children()
                    .flat_map(|x| to_text(ctx, &x, false, parts))
                    .collect::<Vec<_>>();
                if i.is_empty() {
                    return;
                }
                i
            }));
            parts.push(k)
        } else if local == "table" {
            let k = Part::Table(
                node.select("tr")
                    .unwrap()
                    .map(|x| {
                        x.as_node()
                            .children()
                            .flat_map(|x| {
                                let o = x.as_element()?;
                                let a = o.name.local.to_string();
                                if a == "td" {
                                    Some((
                                        false,
                                        TextCompound::Array({
                                            let i = x
                                                .children()
                                                .flat_map(|x| to_text(ctx, &x, false, parts))
                                                .collect::<Vec<_>>();
                                            if i.is_empty() {
                                                return None;
                                            }
                                            i
                                        }),
                                    ))
                                } else if a == "th" {
                                    Some((
                                        true,
                                        TextCompound::Array({
                                            let i = x
                                                .children()
                                                .flat_map(|x| to_text(ctx, &x, false, parts))
                                                .collect::<Vec<_>>();
                                            if i.is_empty() {
                                                return None;
                                            }
                                            i
                                        }),
                                    ))
                                } else {
                                    None
                                }
                            })
                            .collect()
                    })
                    .collect(),
            );
            parts.push(k)
        } else if local == "h4" {
            let o = Part::H(
                Header::H4,
                TextCompound::Array({
                    let i = node
                        .children()
                        .flat_map(|x| to_text(ctx, &x, false, parts))
                        .collect::<Vec<_>>();
                    if i.is_empty() {
                        return;
                    }
                    i
                }),
            );
            parts.push(o)
        } else if local == "h5" {
            let o = Part::H(
                Header::H5,
                TextCompound::Array({
                    let i = node
                        .children()
                        .flat_map(|x| to_text(ctx, &x, false, parts))
                        .collect::<Vec<_>>();
                    if i.is_empty() {
                        return;
                    }
                    i
                }),
            );
            parts.push(o)
        } else if local == "p" {
            let o = Part::P(TextCompound::Array({
                let i = node
                    .children()
                    .flat_map(|x| to_text(ctx, &x, false, parts))
                    .collect::<Vec<_>>();
                if i.is_empty() {
                    return;
                }
                i
            }));
            parts.push(o)
        } else if local == "div"
            || local == "header"
            || local == "section"
            || local == "picture"
            || local == "article"
            || local == "footer"
            || local == "aside"
            || local == "main"
            || local == "q"
        {
            node.children().for_each(|x| parse(ctx, &x, parts));
        } else if local == "ul" {
            let o = Part::Ul(
                node.children()
                    .flat_map(|x| {
                        if x.as_element()?.name.local.to_string() == "li" {
                            Some(TextCompound::Array({
                                let i = x
                                    .children()
                                    .flat_map(|x| to_text(ctx, &x, false, parts))
                                    .collect::<Vec<_>>();
                                if i.is_empty() {
                                    return None;
                                }
                                i
                            }))
                        } else {
                            None
                        }
                    })
                    .collect(),
            );
            parts.push(o)
        } else if local == "i"
            || local == "b"
            || local == "a"
            || local == "abbr"
            || local == "span"
            || local == "sub"
            || local == "sup"
            || local == "strong"
            || local == "em"
            || local == "blockquote"
            || local == "quote"
            || local == "figure"
        {
            if let Some(e) = to_text(ctx, node, false, parts) {
                parts.push(Part::PlainText(e));
            }
        } else {
            println!("Invalid element : {}", local);
        }
    } else if let Some(e) = node.as_text() {
        parts.push(Part::PlainText(TextCompound::Raw(Cow::Owned(
            e.borrow().to_owned(),
        ))))
    } else if node.as_comment().is_none() {
        println!("ERROR NODE")
    }
}

pub fn contains_img(node: &NodeRef) -> bool {
    if let Some(e) = node.as_element() {
        if e.name.local.to_string() == "img" {
            return true;
        }
    }
    for i in node.children() {
        if contains_img(&i) {
            return true;
        }
    }
    false
}

/**
Parses text html (a, p, strong...) to TextCompound pushing non text elements like images in parts.
*/
pub fn to_text<'a>(
    ctx: &Context,
    node: &NodeRef,
    need_check: bool,
    parts: &'a mut Vec<Part<'static>>,
) -> Option<TextCompound<'static>> {
    if let Some(e) = node.as_text() {
        Some(TextCompound::Raw(Cow::Owned(
            e.borrow().replace("\n", "").replace("\t", ""),
        )))
    } else if let Some(e) = node.as_element() {
        let local = e.name.local.to_string();
        for k in ELEMENTS_TO_IGNORE {
            if &local == k {
                return None;
            }
        }
        if !node.text_contents().trim().is_empty()
            && need_check
            && !valid_text(&node.text_contents(), ctx, &local)
        {
            return None;
        }
        if e.attributes.borrow().contains("data-move-to")
            || e.attributes.borrow().contains("hidden")
        {
            return None;
        }
        if local == "i" || local == "em" {
            Some(TextCompound::Italic(Box::new(TextCompound::Array({
                let i = node
                    .children()
                    .flat_map(|x| to_text(ctx, &x, false, parts))
                    .collect::<Vec<_>>();
                if i.is_empty() {
                    return None;
                }
                i
            }))))
        } else if local == "code"
            || local == "pre"
            || (local == "div"
                && e.attributes
                    .borrow()
                    .get("class")
                    .map(|x| x.contains("highlight"))
                    .unwrap_or(false))
        {
            Some(TextCompound::Code(Cow::Owned(code_from_div(node))))
        } else if local == "b" || local == "strong" {
            Some(TextCompound::Bold(Box::new(TextCompound::Array({
                let i = node
                    .children()
                    .flat_map(|x| to_text(ctx, &x, false, parts))
                    .collect::<Vec<_>>();
                if i.is_empty() {
                    return None;
                }
                i
            }))))
        } else if local == "time" || local == "p" {
            Some(TextCompound::Array({
                let i = node
                    .children()
                    .flat_map(|x| to_text(ctx, &x, false, parts))
                    .collect::<Vec<_>>();
                if i.is_empty() {
                    return None;
                }
                i
            }))
        } else if local == "br" || local == "wbr" {
            Some(TextCompound::Br)
        } else if local == "small" {
            Some(TextCompound::Small(Box::new(TextCompound::Array({
                let i = node
                    .children()
                    .flat_map(|x| to_text(ctx, &x, false, parts))
                    .collect::<Vec<_>>();
                if i.is_empty() {
                    return None;
                }
                i
            }))))
        } else if local == "a" {
            if let Some(e) = e.attributes.borrow().get("href") {
                Some(TextCompound::Link(
                    Box::new(TextCompound::Array({
                        let i = node
                            .children()
                            .flat_map(|x| to_text(ctx, &x, false, parts))
                            .collect::<Vec<_>>();
                        if i.is_empty() {
                            return None;
                        }
                        i
                    })),
                    Cow::Owned(e.to_owned()),
                ))
            } else {
                Some(TextCompound::Array({
                    let i = node
                        .children()
                        .flat_map(|x| to_text(ctx, &x, false, parts))
                        .collect::<Vec<_>>();
                    if i.is_empty() {
                        return None;
                    }
                    i
                }))
            }
        } else if local == "span" || local == "q" {
            Some(TextCompound::Array({
                let i = node
                    .children()
                    .flat_map(|x| to_text(ctx, &x, false, parts))
                    .collect::<Vec<_>>();
                if i.is_empty() {
                    return None;
                }
                i
            }))
        } else if local == "abbr" {
            Some(TextCompound::Abbr(
                Box::new(TextCompound::Array({
                    let i = node
                        .children()
                        .flat_map(|x| to_text(ctx, &x, false, parts))
                        .collect::<Vec<_>>();
                    if i.is_empty() {
                        return None;
                    }
                    i
                })),
                Cow::Owned(e.attributes.borrow().get("title").unwrap().to_owned()),
            ))
        } else if local == "sub" {
            Some(TextCompound::Sub(Box::new(TextCompound::Array({
                let i = node
                    .children()
                    .flat_map(|x| to_text(ctx, &x, false, parts))
                    .collect::<Vec<_>>();
                if i.is_empty() {
                    return None;
                }
                i
            }))))
        } else if local == "sup" {
            Some(TextCompound::Sup(Box::new(TextCompound::Array({
                let i = node
                    .children()
                    .flat_map(|x| to_text(ctx, &x, false, parts))
                    .collect::<Vec<_>>();
                if i.is_empty() {
                    return None;
                }
                i
            }))))
        } else if local == "cite" || local == "blockquote" {
            Some(TextCompound::Array({
                let mut i = node
                    .children()
                    .flat_map(|x| to_text(ctx, &x, false, parts))
                    .collect::<Vec<_>>();
                if i.is_empty() {
                    return None;
                }
                i.insert(0, TextCompound::Raw(Cow::Owned("\"".to_owned())));
                i.push(TextCompound::Raw(Cow::Owned("\"".to_owned())));
                i
            }))
        } else if local == "img" {
            if let Some(p) = get_img_link(ctx, &*e.attributes.borrow()) {
                Some(TextCompound::Img(p))
            } else {
                println!("Invalid image {:?}", e);
                None
            }
        } else if local == "h3" || local == "h2" || local == "h1" || local == "h4" || local == "h5"
        {
            let o = Part::H(
                if local == "h3" {
                    Header::H3
                } else if local == "h2" {
                    Header::H2
                } else if local == "h1" {
                    Header::H1
                } else if local == "h4" {
                    Header::H4
                } else {
                    Header::H5
                },
                TextCompound::Array({
                    let i = node
                        .children()
                        .flat_map(|x| to_text(ctx, &x, false, parts))
                        .collect::<Vec<_>>();
                    if i.is_empty() {
                        return None;
                    }
                    i
                }),
            );
            parts.push(o);
            None
        } else if local == "div"
            || local == "figure"
            || local == "ul"
            || local == "li"
            || local == "ol"
            || local == "picture"
            || local == "header"
        {
            Some(TextCompound::Array({
                let i = node
                    .children()
                    .flat_map(|x| to_text(ctx, &x, true, parts))
                    .collect::<Vec<_>>();
                if i.is_empty() {
                    return None;
                }
                i
            }))
        } else {
            println!("ERROR {}", local);
            None
        }
    } else {
        if node.as_comment().is_none() {
            println!("B {:?}", node);
        }
        None
    }
}
