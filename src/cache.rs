use std::{fs::OpenOptions, io::Write};

use crate::text_parser::Context;
use anyhow::*;
use dashmap::DashMap;
use kuchiki::traits::TendrilSink;
use once_cell::sync::Lazy;
use reqwest::Url;

use crate::{
    text_parser,
    title_extractor::try_extract_data,
    utils::{gen_html, http_get, remove, sha256},
};

const URLS: Lazy<DashMap<String, String>> = Lazy::new(|| {
    if let Some(e) = std::fs::read_to_string("db.json").ok() {
        e.lines()
            .filter(|x| x.contains("|"))
            .map(|x| {
                let barre = x.find('|').unwrap();
                (x[0..barre].to_owned(), x[barre + 1..].to_owned())
            })
            .collect()
    } else {
        DashMap::new()
    }
});

pub fn get_url_for_shortened(shortened: &str) -> Option<String> {
    URLS.get(shortened).map(|x| x.as_str().to_owned())
}

pub fn get_shortened_from_url(url: &str) -> String {
    let short = sha256(url);
    let short = &short[..6];
    if !URLS.contains_key(short) {
        URLS.insert(short.to_owned(), url.to_owned());
        OpenOptions::new()
            .append(true)
            .create(true)
            .write(true)
            .open("db.json")
            .unwrap()
            .write_all(format!("{}|{}\n", short, url).as_bytes())
            .unwrap();
    }
    short.to_owned()
}

pub fn get_file(url: &str) -> Result<String> {
    let cache_file = format!("cache/{}.html", sha256(url));
    Ok(if let Some(e) = std::fs::read_to_string(&cache_file).ok() {
        e
    } else {
        let html = get_html(url)?;
        std::fs::write(cache_file, &html)?;
        html
    })
}

fn get_html(url: &str) -> Result<String> {
    let document = kuchiki::parse_html().one(http_get(url)?);
    [
        ".capping",
        "script",
        "style",
        ".comment-list",
        ".comment",
        ".comments",
        ".related-story",
        ".article__aside",
        ".sgt-inread",
        ".abo-inread",
        "#placeholder--inread",
        "div.rebond",
        ".ads",
        ".ad",
        ".advertisement",
        ".pub",
        "aside",
        "#comments",
        "#comment",
    ]
    .iter()
    .for_each(|x| remove(&document, x));
    let h = try_extract_data(&document);
    let ctx = Context {
        meta: h,
        url: Url::parse(url)?,
    };
    Ok(gen_html(
        &text_parser::clean_html(&document.select_first("body").unwrap().as_node(), &ctx),
        &ctx,
    ))
}
