use std::path::Path;

use once_cell::sync::Lazy;

#[derive(serde::Deserialize)]
pub struct Config {
    pub enable_cache: bool,
    pub recompress_images: bool,
    pub cache_folder: String,
    pub database_file: String,
    pub address: String,
    pub max_size: u64,
}

/// Default config written out the first time the server starts in a fresh
/// working directory. Kept inline so reader-core doesn't need to reach back
/// up into the workspace root for a config file.
const DEFAULT_CONFIG: &str = r#"enable_cache = false
recompress_images = true
cache_folder = "data/cache"
database_file = "data/db.sqlite"
address = "127.0.0.1:8080"
max_size = 8048576
"#;

pub static CONFIG: Lazy<Config> = Lazy::new(|| {
    let path = Path::new("config.toml");
    if !path.exists() {
        std::fs::write(path, DEFAULT_CONFIG).expect("write default config.toml");
    }
    let contents = std::fs::read_to_string(path).expect("read config.toml");
    toml::from_str(&contents).expect("parse config.toml")
});
