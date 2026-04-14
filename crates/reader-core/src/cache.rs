//! URL short-id store + optional on-disk HTML cache.
//!
//! This module owns nothing beyond a SQLite connection and a small
//! filesystem helper. It does not know about the rendering pipeline — the
//! server orchestrates "cache miss → call page actor → store result".

use std::sync::Mutex;

use once_cell::sync::Lazy;
use rusqlite::{params, Connection, OptionalExtension};

use crate::{
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

/// Is on-disk HTML caching enabled by config?
pub fn is_enabled() -> bool {
    CONFIG.enable_cache
}

/// Deterministic cache path for an article URL.
pub fn cache_path(url: &str) -> String {
    format!("{}/{}.html", CONFIG.cache_folder, sha256(url))
}

/// Try to read a cached render from disk. Returns `Ok(None)` on a miss so
/// the caller can tell that apart from a hard I/O error.
pub async fn try_cached(url: &str) -> Result<Option<String>> {
    if !is_enabled() {
        return Ok(None);
    }
    match tokio::fs::read_to_string(cache_path(url)).await {
        Ok(html) => Ok(Some(html)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(Error::Io(e)),
    }
}

/// Write a rendered article to disk. Best-effort: a failed write is logged
/// but does not propagate, because the in-memory response is still valid.
pub async fn store(url: &str, html: &str) {
    if !is_enabled() {
        return;
    }
    let path = cache_path(url);
    if let Some(parent) = std::path::Path::new(&path).parent() {
        if let Err(e) = tokio::fs::create_dir_all(parent).await {
            eprintln!("mkdir cache {}: {}", parent.display(), e);
            return;
        }
    }
    if let Err(e) = tokio::fs::write(&path, html).await {
        eprintln!("cache write {}: {}", path, e);
    }
}
