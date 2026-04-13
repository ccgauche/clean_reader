use std::collections::HashMap;

mod cache;
mod config;
mod error;
mod html_node;
mod image;
mod score_implementation;
mod text_element;
mod text_parser;
mod title_extractor;
mod utils;

use crate::error::{Error, Result};

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
pub fn run_v2(url: &str, min_id: &str, other_download: bool) -> Result<String> {
    use html5ever::tendril::TendrilSink;
    use std::io::Cursor;

    let httpstring = crate::utils::http_get(url)?;
    let httpstring = if let Some(e) = httpstring.find("rel=\"amphtml\"") {
        let k = &httpstring[(e + "rel=\"amphtml\"".len())..];
        let e = k.split('"').nth(1).unwrap();
        println!("Using AMPHTML");
        crate::utils::http_get(e)?
    } else {
        httpstring
    };

    // First parse: pull og:title, og:image, <title> from the full document.
    let metadata_dom = html5ever::parse_document(
        markup5ever_rcdom::RcDom::default(),
        html5ever::ParseOpts::default(),
    )
    .one(httpstring.clone());
    let mut meta = crate::title_extractor::try_extract_data(&metadata_dom.document);
    drop(metadata_dom);

    // Readability (Firefox reader-view algorithm) picks the article subtree.
    let parsed_url = reqwest::Url::parse(url).map_err(|e| Error::InvalidUrl(e.to_string()))?;
    let mut cursor = Cursor::new(httpstring);
    let product = readability::extractor::extract(&mut cursor, &parsed_url)
        .map_err(|e| Error::Readability(e.to_string()))?;
    if meta.title.is_none() && !product.title.is_empty() {
        meta.title = Some(product.title);
    }

    // Lower Readability's cleaned HTML into our HTMLNode / TextCompound pipeline.
    let content_dom = html5ever::parse_document(
        markup5ever_rcdom::RcDom::default(),
        html5ever::ParseOpts::default(),
    )
    .one(product.content);
    let html = HTMLNode::from_handle(&content_dom.document)?;

    let mut ctx = crate::text_parser::Context {
        meta,
        download: other_download,
        min_id: min_id.to_string(),
        url: parsed_url,
        map: HashMap::new(),
        count: 0,
    };
    let text = TextCompound::from_node(&mut ctx, &html).ok_or(Error::EmptyArticle)?;
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
    if contains_image(&html).0 {
        ctx.meta.image = None;
    }
    let k = gen_html_2(&[text], &mut ctx);
    Ok(k)
}

#[get("/r/{base64url}")]
async fn index_r(base64url: web::Path<String>) -> HttpResponse {
    let output: Result<String> = (|| {
        let url = String::from_utf8(base64::decode(base64url.replace('_', "/"))?)?;
        Ok(get_shortened_from_url(&url))
    })();
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
    let short = short.into_inner();
    let output: Result<String> = web::block(move || {
        let url = get_url_for_shortened(&short).ok_or(Error::UnknownShortId)?;
        println!("{}", url);
        get_file(&url, &short, false)
    })
    .await
    .map_err(Error::from)
    .and_then(|r| r);
    match output {
        Ok(e) => HttpResponse::Ok().content_type("text/html").body(e),
        Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
    }
}
#[get("/i/{short}")]
async fn index_i(short: web::Path<String>) -> HttpResponse {
    let output: Result<Vec<u8>> =
        std::fs::read(format!("cache/images/{}.avif", short)).map_err(Error::from);
    match output {
        Ok(e) => HttpResponse::Ok().content_type("image/avif").body(e),
        Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
    }
}

#[get("/d/{short}")]
async fn download(short: web::Path<String>) -> HttpResponse {
    let short = short.into_inner();
    let output: Result<String> = web::block(move || {
        let url = get_url_for_shortened(&short).ok_or(Error::UnknownShortId)?;
        println!("{}", url);
        get_file(&url, &short, true)
    })
    .await
    .map_err(Error::from)
    .and_then(|r| r);
    match output {
        Ok(e) => HttpResponse::Ok().content_type("text/html").body(e),
        Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let base = format!("http://{}", CONFIG.address);
    let example_url = "https://en.wikipedia.org/wiki/Computer";
    let example_encoded = base64::encode(example_url).replace('/', "_");
    println!("Clean Reader listening on {}", base);
    println!("Try it: {}/r/{}  ({})", base, example_encoded, example_url);
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
