use std::process::exit;

use kuchiki::traits::TendrilSink;
use reqwest::Url;
use utils::gen_md;

use crate::{
    text_parser::Context,
    title_extractor::try_extract_data,
    utils::{gen_html, http_get, remove},
};

mod structures;
mod text_parser;
mod title_extractor;
mod utils;

fn main() {
    let args = std::env::args();
    if args.len() != 3 {
        println!("expected <OUTPUT FILE (.html or .md)> <URL>");
        exit(0);
    }
    let mut args = std::env::args().skip(1);
    let arg1 = args.next().unwrap();
    let arg2 = args.next().unwrap();
    let url = &arg2;
    let document = kuchiki::parse_html().one(http_get(url));
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
        url: Url::parse(url).unwrap(),
    };
    if arg1.ends_with(".html") {
        gen_html(
            &text_parser::clean_html(&document.select_first("body").unwrap().as_node(), &ctx),
            &ctx,
            &arg1,
        );
    } else {
        gen_md(
            &text_parser::clean_html(&document.select_first("body").unwrap().as_node(), &ctx),
            &ctx,
            &arg1,
        );
    }
}
