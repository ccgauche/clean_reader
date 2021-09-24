#![feature(try_blocks)]
#![feature(box_syntax)]
use anyhow::*;

mod cache;
mod new_arch;
mod text_parser;
mod title_extractor;
mod utils;

use actix_web::{get, web, App, HttpResponse, HttpServer};
use cache::{get_file, get_shortened_from_url};

use crate::cache::get_url_for_shortened;

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
        get_file(&url)?
    };
    match output {
        Ok(e) => HttpResponse::Ok().content_type("text/html").body(e),
        Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
    }
}

#[get("/dist/index.css")]
async fn index_css() -> HttpResponse {
    HttpResponse::Ok()
        .content_type("text/css")
        .body(include_str!("../dist/index.css"))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .service(index_r)
            .service(index_m)
            .service(index_css)
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
