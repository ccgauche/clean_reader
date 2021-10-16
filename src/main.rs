#![feature(try_blocks)]
#![feature(box_syntax)]
use std::collections::HashMap;

use anyhow::*;

mod cache;
mod config;
mod html_node;
mod score;
mod text_element;
mod text_parser;
mod title_extractor;
mod utils;

use actix_web::{get, web, App, HttpResponse, HttpServer};
use cache::{get_file, get_shortened_from_url};

use crate::{
    cache::get_url_for_shortened, config::CONFIG, html_node::HTMLNode, score::best_node,
    text_element::TextCompound, utils::gen_html_2,
};

pub fn run_v2(url: &str, min_id: &str, other_download: bool) -> anyhow::Result<String> {
    use kuchiki::traits::TendrilSink;
    let document = kuchiki::parse_html().one(crate::utils::http_get(url)?);
    let h = crate::title_extractor::try_extract_data(&document);
    let html = HTMLNode::from_node_ref(document)
        .ok_or_else(|| anyhow::anyhow!("Invalid HTMLNode ref generation"))?;
    let mut ctx = crate::text_parser::Context {
        meta: h,
        download: other_download,
        min_id: min_id.to_string(),
        url: reqwest::Url::parse(url).expect("Invalid URL"),
        map: HashMap::new(),
        count: 0,
    };
    let text = TextCompound::from_html_node(&mut ctx, best_node(&html))
        .ok_or_else(|| anyhow::anyhow!("Invalid HTML generation"))?;
    let text = if ctx.meta.title.is_some() {
        text.remove_title()
    } else {
        text
    };
    std::fs::write(&CONFIG.text_element_debug_file, text.to_string()).unwrap();
    Ok(gen_html_2(&[text], &ctx))
}

#[get("/r/{base64url}")]
async fn index_r(web::Path(base64url): web::Path<String>) -> HttpResponse {
    let output: Result<String> = try {
        let url = String::from_utf8(base64::decode(base64url.replace("_", "/"))?)?;
        get_shortened_from_url(&url)
    };
    match output {
        Ok(e) => HttpResponse::MovedPermanently()
            .set_header("location", format!("/m/{}", e))
            .body(""),
        Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
    }
}

#[get("/m/{short}")]
async fn index_m(web::Path(short): web::Path<String>) -> HttpResponse {
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

#[get("/d/{short}")]
async fn download(web::Path(short): web::Path<String>) -> HttpResponse {
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
            .service(download)
    })
    .bind(&CONFIG.address)?
    .run()
    .await
}
