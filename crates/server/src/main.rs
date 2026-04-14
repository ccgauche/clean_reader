use actix_web::{get, web, App, HttpResponse, HttpServer};
use reader_core::cache::{self, get_shortened_from_url, get_url_for_shortened};
use reader_core::config::CONFIG;
use reader_core::error::{Error, Result};
use reader_core::RenderMode;
use tokio::fs;

#[get("/r/{base64url}")]
async fn index_r(base64url: web::Path<String>) -> HttpResponse {
    let output: Result<String> = (|| {
        let url = String::from_utf8(base64::decode(base64url.replace('_', "/"))?)?;
        get_shortened_from_url(&url)
    })();
    match output {
        Ok(short) => HttpResponse::MovedPermanently()
            .insert_header(("location", format!("/m/{}", short)))
            .body(""),
        Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
    }
}

/// Resolve a short id to a URL, serve from the disk cache if enabled,
/// else ask the page actor to render it and store the result.
async fn serve_short(short: String, mode: RenderMode) -> HttpResponse {
    let output: Result<String> = async {
        let url = get_url_for_shortened(&short)?.ok_or(Error::UnknownShortId)?;
        println!("{}", url);
        if let Some(cached) = cache::try_cached(&url).await? {
            return Ok(cached);
        }
        let rendered = page_actor::render_page(&url, &short, mode).await?;
        cache::store(&url, &rendered).await;
        Ok(rendered)
    }
    .await;
    match output {
        Ok(html) => HttpResponse::Ok().content_type("text/html").body(html),
        Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
    }
}

#[get("/m/{short}")]
async fn index_m(short: web::Path<String>) -> HttpResponse {
    serve_short(short.into_inner(), RenderMode::View).await
}

#[get("/i/{short}")]
async fn index_i(short: web::Path<String>) -> HttpResponse {
    let path = format!("{}/images/{}.avif", CONFIG.cache_folder, short.into_inner());
    match fs::read(path).await {
        Ok(bytes) => HttpResponse::Ok().content_type("image/avif").body(bytes),
        Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
    }
}

#[get("/d/{short}")]
async fn download(short: web::Path<String>) -> HttpResponse {
    serve_short(short.into_inner(), RenderMode::Download).await
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let base = format!("http://{}", CONFIG.address);
    let example_url = "https://en.wikipedia.org/wiki/Computer";
    let example_encoded = base64::encode(example_url).replace('/', "_");
    println!("Clean Reader listening on {}", base);
    println!("Try it: {}/r/{}  ({})", base, example_encoded, example_url);

    if let Err(e) = image_actor::boot().await {
        eprintln!("failed to start image actor: {}", e);
        return Err(std::io::Error::other(e.to_string()));
    }
    if let Err(e) = page_actor::boot().await {
        eprintln!("failed to start page actor: {}", e);
        return Err(std::io::Error::other(e.to_string()));
    }

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
