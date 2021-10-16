use once_cell::sync::Lazy;

#[derive(serde::Deserialize)]
pub struct Config {
    pub enable_debug_text_element: bool,
    pub enable_cache: bool,
    pub cache_folder: String,
    pub database_file: String,
    pub text_element_debug_file: String,
    pub address: String,
}

pub static CONFIG: Lazy<Config> =
    Lazy::new(|| toml::from_str(&std::fs::read_to_string("config.toml").unwrap()).unwrap());
