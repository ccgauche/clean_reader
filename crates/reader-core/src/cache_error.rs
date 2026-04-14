/// Errors from the on-disk cache + SQLite URL store.
#[derive(Debug, thiserror::Error)]
pub enum CacheError {
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("URL store mutex poisoned")]
    MutexPoisoned,

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}
