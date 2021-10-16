use std::collections::HashMap;

use crate::{
    config::CONFIG,
    new_arch::{html_node::HTMLNode, score::best_node},
    utils::gen_html_2,
};

pub mod html_node;
mod score;
pub mod text_element;
mod text_parser;

pub fn filter_names(string: &str) -> &str {
    if string.contains("img") {
        "img"
    } else if string.contains("source") {
        "source"
    } else {
        string
    }
}

pub fn run_v2(url: &str, min_id: &str, download: bool) -> anyhow::Result<String> {
    use kuchiki::traits::TendrilSink;
    let document = kuchiki::parse_html().one(crate::utils::http_get(url)?);
    let h = crate::title_extractor::try_extract_data(&document);
    let html = HTMLNode::from_node_ref(document)
        .ok_or_else(|| anyhow::anyhow!("Invalid HTMLNode ref generation"))?;
    let mut ctx = crate::text_parser::Context {
        meta: h,
        download,
        min_id: min_id.to_string(),
        url: reqwest::Url::parse(url).expect("Invalid URL"),
        map: HashMap::new(),
        count: 0,
    };
    let text = text_element::TextCompound::from_html_node(&mut ctx, best_node(&html))
        .ok_or_else(|| anyhow::anyhow!("Invalid HTML generation"))?;
    let text = if ctx.meta.title.is_some() {
        text.remove_title()
    } else {
        text
    };
    std::fs::write(&CONFIG.text_element_debug_file, text.to_string()).unwrap();
    Ok(gen_html_2(&[text], &ctx))
}
