#![feature(try_blocks)]
#![feature(box_syntax)]
use std::collections::HashMap;

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
    let httpstring = crate::utils::http_get(url)?;
    let httpstring = if let Some(e) = httpstring.find("rel=\"amphtml\"") {
        let k = &httpstring[(e + "rel=\"amphtml\"".len())..];
        let e = k.split('"').nth(1).unwrap();
        println!("Using AMPHTML");
        crate::utils::http_get(e)?
    } else {
        httpstring
    };
    let document = kuchiki::parse_html().one(httpstring);
    let h = crate::title_extractor::try_extract_data(&document);
    let html = HTMLNode::from_node_ref(document)?;
    let mut ctx = crate::text_parser::Context {
        meta: h,
        download: other_download,
        min_id: min_id.to_string(),
        url: reqwest::Url::parse(url).expect("Invalid URL"),
        map: HashMap::new(),
        count: 0,
    };
    let node = choose(&html);
    let text = TextCompound::from_node(&mut ctx, node)
        .ok_or_else(|| anyhow::anyhow!("Invalid HTML generation"))?;
    if ctx.meta.title.is_none() {
        ctx.meta.title = extract_title(&html, node).1;
    }
    let text = if ctx.meta.title.is_some() {
        text.remove_title()
    } else {
        text
    };
    let mut string = String::new();
    text.json(&mut string);
    while string.contains(",,") {
        string = string.replace(",,", ",");
    }
    while string.contains(",]") {
        string = string.replace(",]", "]");
    }
    while string.contains("[,") {
        string = string.replace("[,", "[");
    }
    if ctx.meta.title.is_none() && !text.contains_title() {
        ctx.meta.title = ctx.meta.etitle.clone();
    };
    if contains_image(node).0 {
        ctx.meta.image = None;
    }
    let k = gen_html_2(&[text], &mut ctx);
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

#[get("/test/{nom}")]
async fn test(nom: web::Path<String>) -> String {
    format!("{} est con", nom)
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
