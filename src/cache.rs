use std::sync::Mutex;

use once_cell::sync::Lazy;
use rusqlite::{params, Connection, OptionalExtension};

use crate::{
    actors,
    config::CONFIG,
    error::{Error, Result},
    utils::sha256,
};

// Startup-time panic is acceptable here: a server with no URL store is
// fundamentally unusable, and a clear failure at boot is easier to diagnose
// than a cascade of "SQLite not initialised" errors at request time.
static DB: Lazy<Mutex<Connection>> = Lazy::new(|| {
    if let Some(parent) = std::path::Path::new(&CONFIG.database_file).parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let conn = Connection::open(&CONFIG.database_file).expect("open sqlite");
    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA synchronous = NORMAL;
         CREATE TABLE IF NOT EXISTS urls (
             short TEXT PRIMARY KEY,
             url   TEXT NOT NULL
         );",
    )
    .expect("init sqlite schema");
    Mutex::new(conn)
});

pub fn get_url_for_shortened(shortened: &str) -> Result<Option<String>> {
    let conn = DB.lock().map_err(|_| Error::DbPoisoned)?;
    Ok(conn
        .query_row(
            "SELECT url FROM urls WHERE short = ?1",
            params![shortened],
            |row| row.get::<_, String>(0),
        )
        .optional()?)
}

pub fn get_shortened_from_url(url: &str) -> Result<String> {
    let short = sha256(url)[..6].to_owned();
    let conn = DB.lock().map_err(|_| Error::DbPoisoned)?;
    conn.execute(
        "INSERT OR IGNORE INTO urls (short, url) VALUES (?1, ?2)",
        params![short, url],
    )?;
    Ok(short)
}

pub async fn get_file(url: &str, min_id: &str, download: bool) -> Result<String> {
    if CONFIG.enable_cache {
        let cache_file = format!("{}/{}.html", CONFIG.cache_folder, sha256(url));
        if let Ok(e) = tokio::fs::read_to_string(&cache_file).await {
            Ok(e)
        } else {
            let html = actors::render_page(url, min_id, download).await?;
            let _ = tokio::fs::write(cache_file, &html).await;
            Ok(html)
        }
    } else {
        actors::render_page(url, min_id, download).await
    }
}
