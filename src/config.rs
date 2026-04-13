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

pub static CONFIG: Lazy<Config> = Lazy::new(|| {
    toml::from_str({
        if !Path::new("config.toml").exists() {
            std::fs::write("config.toml", include_str!("../config.toml")).unwrap();
        }
        &std::fs::read_to_string("config.toml").unwrap()
    })
    .unwrap()
});
