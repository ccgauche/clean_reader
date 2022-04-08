#![feature(try_blocks)]
#![feature(box_syntax)]
use std::collections::HashMap;

use flame::Library;

use anyhow::*;

mod cache;
mod config;
mod html_node;
mod image;
mod score_implementation;
mod text_element;
mod text_parser;
mod title_extractor;
mod utils;

use actix_web::{get, web, App, HttpResponse, HttpServer};
use cache::{get_file, get_shortened_from_url};

use crate::{
    cache::get_url_for_shortened, config::CONFIG, html_node::HTMLNode, score_implementation::*,
    text_element::TextCompound, utils::gen_html_2,
};

/**
 * This is the main function which is called when a page is accessed.
 * It will parse the page and return the content as string.
 */
pub fn run_v2(url: &str, min_id: &str, other_download: bool) -> anyhow::Result<String> {
    use kuchiki::traits::TendrilSink;
    let mut library = Library::new();
    library.start("handle webpage");

    library.start("http get");
    let httpstring = crate::utils::http_get(url)?;
    library.end("http get");
    library.start("amp html");
    let httpstring = if let Some(e) = httpstring.find("rel=\"amphtml\"") {
        let k = &httpstring[(e + "rel=\"amphtml\"".len())..];
        let e = k.split('"').nth(1).unwrap();
        println!("Using AMPHTML");
        library.start("amp html fetch");
        let k = crate::utils::http_get(e)?;
        library.end("amp html fetch");
        k
    } else {
        httpstring
    };
    library.end("amp html");
    library.start("page processing");
    library.start("html parse");
    let document = kuchiki::parse_html().one(httpstring);
    library.end("html parse");
    library.start("head data extractor");
    let h = crate::title_extractor::try_extract_data(&document);
    library.end("head data extractor");
    library.start("html tree cleanup");
    let html = HTMLNode::from_node_ref(document)?;
    library.end("html tree cleanup");
    let mut ctx = crate::text_parser::Context {
        library,
        meta: h,
        download: other_download,
        min_id: min_id.to_string(),
        url: reqwest::Url::parse(url).expect("Invalid URL"),
        map: HashMap::new(),
        count: 0,
    };
    ctx.library.start("content extraction");
    let node = choose(&html);
    ctx.library.end("content extraction");
    ctx.library.start("html tree reformating");
    let text = TextCompound::from_node(&mut ctx, node)
        .ok_or_else(|| anyhow::anyhow!("Invalid HTML generation"))?;
    ctx.library.end("html tree reformating");
    if ctx.meta.title.is_none() {
        ctx.library.start("title extraction");
        ctx.meta.title = extract_title(&html, node).1;
        ctx.library.end("title extraction");
    }
    ctx.library.start("title elision");
    let text = if ctx.meta.title.is_some() {
        text.remove_title()
    } else {
        text
    };
    ctx.library.end("title elision");
    ctx.library.start("title replacement");
    if ctx.meta.title.is_none() && !text.contains_title() {
        ctx.meta.title = ctx.meta.etitle.clone();
    };
    ctx.library.end("title replacement");
    ctx.library.start("image duplication check");
    if contains_image(node).0 {
        ctx.meta.image = None;
    }
    ctx.library.end("image duplication check");
    ctx.library.start("html generation");
    let k = gen_html_2(&[text], &mut ctx);
    ctx.library.end("html generation");
    ctx.library.end("page processing");
    ctx.library.end("handle webpage");
    if CONFIG.bench_mode {
        ctx.library
            .dump(std::fs::File::create("flamegraph.html").unwrap())
            .unwrap();
    };
    Ok(k)
}

#[get("/r/{base64url}")]
async fn index_r(base64url: web::Path<String>) -> HttpResponse {
    let output: Result<String> = try {
        let url = String::from_utf8(base64::decode(base64url.replace('_', "/"))?)?;
        get_shortened_from_url(&url)
    };
    match output {
        Ok(e) => HttpResponse::MovedPermanently()
            .insert_header(("location", format!("/m/{}", e)))
            .body(""),
        Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
    }
}

#[get("/m/{short}")]
async fn index_m(short: web::Path<String>) -> HttpResponse {
    let output: Result<String> = try {
        let url =
            get_url_for_shortened(&short).ok_or_else(|| anyhow!("Can't find url in database"))?;
        println!("{}", url);
        get_file(&url, &short, false)?
    };
    match output {
        Ok(e) => HttpResponse::Ok().content_type("text/html").body(e),
        Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
    }
}
#[get("/i/{short}")]
async fn index_i(short: web::Path<String>) -> HttpResponse {
    let output: Result<Vec<u8>> = try { std::fs::read(format!("cache/images/{}.avif", short))? };
    match output {
        Ok(e) => HttpResponse::Ok().content_type("image/avif").body(e),
        Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
    }
}

#[get("/d/{short}")]
async fn download(short: web::Path<String>) -> HttpResponse {
    let output: Result<String> = try {
        let url =
            get_url_for_shortened(&short).ok_or_else(|| anyhow!("Can't find url in database"))?;
        println!("{}", url);
        get_file(&url, &short, true)?
    };
    match output {
        Ok(e) => HttpResponse::Ok().content_type("text/html").body(e),
        Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .service(index_r)
            .service(index_m)
            .service(index_i)
            .service(download)
    })
    .bind(&CONFIG.address)?
    .run()
    .await
}
