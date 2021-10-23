use std::{borrow::Cow, collections::HashMap};

use kuchiki::NodeRef;
use reqwest::Url;

use crate::{bench::Monitor, title_extractor::ArticleData};

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
The context of the parser (The current url for link absolutization and the article data to avoid including multiple time the same title)
*/
pub struct Context<'a> {
    pub bench: Monitor,
    pub url: Url,
    pub download: bool,
    pub min_id: String,
    pub map: HashMap<&'a str, usize>,
    pub count: usize,
    pub meta: ArticleData,
}

impl<'a> Context<'a> {
    pub fn absolutize(&mut self, url: &'a str) -> Cow<'a, str> {
        if let Some(k) = url.strip_prefix('#') {
            if let Some(mk) = self.map.get(k) {
                Cow::Owned(format!("#{}", mk))
            } else {
                self.count += 1;
                self.map.insert(k, self.count);
                Cow::Owned(format!("#{}", self.count))
            }
        } else {
            self.url
                .join(url)
                .map(|x| Cow::Owned(x.to_string()))
                .unwrap_or_else(|_| Cow::Borrowed(url))
        }
    }
}
