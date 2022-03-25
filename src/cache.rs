use std::{fs::OpenOptions, io::Write};

use crate::{config::CONFIG, run_v2};
use anyhow::*;
use dashmap::DashMap;

use once_cell::sync::Lazy;

use crate::utils::sha256;

static URLS: Lazy<DashMap<String, String>> = Lazy::new(|| {
    if let Ok(e) = std::fs::read_to_string(&CONFIG.database_file) {
        e.lines()
            .filter(|x| x.contains('|'))
            .map(|x| {
                let barre = x.find('|').unwrap();
                (x[0..barre].to_owned(), x[barre + 1..].to_owned())
            })
            .collect()
    } else {
        DashMap::new()
    }
});

pub fn get_url_for_shortened(shortened: &str) -> Option<String> {
    URLS.get(shortened).map(|x| x.as_str().to_owned())
}

pub fn get_shortened_from_url(url: &str) -> String {
    let short = sha256(url);
    let short = &short[..6];
    if !URLS.contains_key(short) {
        URLS.insert(short.to_owned(), url.to_owned());
        OpenOptions::new()
            .append(true)
            .create(true)
            .write(true)
            .open(&CONFIG.database_file)
            .unwrap()
            .write_all(format!("{}|{}\n", short, url).as_bytes())
            .unwrap();
    }
    short.to_owned()
}

pub fn get_file(url: &str, min_id: &str, download: bool) -> Result<String> {
    if CONFIG.enable_cache {
        let cache_file = format!("{}/{}.html", CONFIG.cache_folder, sha256(url));
        Ok(if let Ok(e) = std::fs::read_to_string(&cache_file) {
            e
        } else {
            let html = run_v2(url, min_id, download)?;
            std::fs::write(cache_file, &html)?;
            html
        })
    } else {
        run_v2(url, min_id, download)
    }
}
