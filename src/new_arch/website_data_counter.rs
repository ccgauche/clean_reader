use std::collections::{HashMap, HashSet};

use anyhow::*;

use kuchiki::{traits::TendrilSink, NodeRef};
use reqwest::Url;

use crate::{
    cache::get_file,
    text_parser::Context,
    title_extractor::{try_images_data, ArticleData},
    utils::get_img_link,
};

pub struct CounterState {
    already_loaded: HashSet<Url>,
    current_url: Url,
    bytes: HashMap<DataType, usize>,
}

#[test]
fn test() {
    fcompare(&"https://www.theguardian.com/society/2021/sep/11/covid-jabs-for-12--to-15-year-olds-set-to-start-in-weeks".parse().unwrap());
}

fn fcompare(url: &Url) {
    let file = get_file(url.as_str()).unwrap();
    std::fs::write("debug-clean.html", &file).unwrap();
    println!("Clean reader size : ");
    let mut c = CounterState::mesure_website_content(
        &file.replace("<link rel=\"stylesheet\" href=\"/dist/index.css\" />", ""),
        url,
    );
    c.add_bytes(DataType::CSS, include_bytes!("../../dist/index.css").len());
    c.display_info();
    println!();
    println!("Original size : ");
    CounterState::mesure_website(url).display_info();
}

impl CounterState {
    pub fn mesure_website_content(string: &str, url: &Url) -> Self {
        let mut this = Self {
            already_loaded: HashSet::new(),
            current_url: url.clone(),
            bytes: HashMap::new(),
        };
        read_html(string, url, &mut this);
        this
    }
    pub fn mesure_website(url: &Url) -> Self {
        let mut this = Self {
            already_loaded: HashSet::new(),
            current_url: url.clone(),
            bytes: HashMap::new(),
        };
        read_html_url(url, &mut this);
        this
    }
    pub fn display_info(&self) {
        println!("====== Data usage analysis ======");
        println!("Website: {}", self.current_url);
        let mut bytes = 0;
        for (a, b) in self.bytes.iter() {
            bytes += *b;
            println!("{:?} => {}Kb", a, *b as f32 / 1000.);
        }
        println!("Total bytes => {}Kb", bytes as f32 / 1000.);
        println!("=================================");
    }
    pub fn add_bytes_url(&mut self, ty: DataType, url: &Url) {
        if self.already_loaded.contains(url) {
            return;
        }
        self.already_loaded.insert(url.clone());
        self.add_bytes(ty, http_get_len(url.as_str()).unwrap_or(0))
    }
    pub fn add_bytes_css(&mut self, url: &Url) {
        if self.already_loaded.contains(url) {
            return;
        }
        self.already_loaded.insert(url.clone());
        // ADD FONT
        let size = http_get_len(url.as_str()).unwrap_or(0);
        self.add_bytes(DataType::CSS, size)
    }
    pub fn add_bytes(&mut self, ty: DataType, bytes: usize) {
        if let Some(e) = self.bytes.get_mut(&ty) {
            *e += bytes;
        } else {
            self.bytes.insert(ty, bytes);
        }
    }
}

#[derive(PartialEq, Eq, Hash, Debug)]
pub enum DataType {
    HTML,
    JS,
    CSS,
    FONT,
    IMG,
}
fn read_html(string: &str, url: &Url, state: &mut CounterState) {
    pub fn inner(nr: &NodeRef, state: &mut CounterState, url: &Url) {
        for i in nr.children() {
            inner(&i, state, url);
        }
        if let Some(e) = nr.as_element() {
            match e.name.local.to_string().as_str() {
                "link" => {
                    if let Some(a) = e.attributes.borrow().get("href") {
                        if let Some(b) = e.attributes.borrow().get("rel") {
                            if b == "stylesheet" {
                                state.add_bytes_css(
                                    &Url::join(url, a).unwrap_or_else(|_| a.parse().unwrap()),
                                );
                            } else if b == "icon" {
                                state.add_bytes_url(
                                    DataType::IMG,
                                    &Url::join(url, a).unwrap_or_else(|_| a.parse().unwrap()),
                                );
                            } else if b == "font" {
                                state.add_bytes_url(
                                    DataType::FONT,
                                    &Url::join(url, a).unwrap_or_else(|_| a.parse().unwrap()),
                                );
                            } else if b == "script" {
                                state.add_bytes_url(
                                    DataType::JS,
                                    &Url::join(url, a).unwrap_or_else(|_| a.parse().unwrap()),
                                );
                            }
                        }
                        if let Some(b) = e.attributes.borrow().get("as") {
                            if b == "stylesheet" {
                                state.add_bytes_css(
                                    &Url::join(url, a).unwrap_or_else(|_| a.parse().unwrap()),
                                );
                            } else if b == "icon" {
                                state.add_bytes_url(
                                    DataType::IMG,
                                    &Url::join(url, a).unwrap_or_else(|_| a.parse().unwrap()),
                                );
                            } else if b == "font" {
                                state.add_bytes_url(
                                    DataType::FONT,
                                    &Url::join(url, a).unwrap_or_else(|_| a.parse().unwrap()),
                                );
                            } else if b == "script" {
                                state.add_bytes_url(
                                    DataType::JS,
                                    &Url::join(url, a).unwrap_or_else(|_| a.parse().unwrap()),
                                );
                            }
                        }
                    }
                }
                "img" | "source" | "picture" => {
                    if let Some(a) = get_img_link(
                        &Context {
                            url: url.clone(),
                            meta: ArticleData {
                                image: None,
                                title: None,
                            },
                        },
                        &e.attributes.borrow(),
                    ) {
                        println!("{}", a);
                        state.add_bytes_url(
                            DataType::IMG,
                            &Url::join(url, a.as_ref()).unwrap_or_else(|_| a.parse().unwrap()),
                        );
                    } else {
                        println!("Skipped img {:?}", e.attributes);
                    }
                }
                "script" => {
                    if let Some(a) = e.attributes.borrow().get("src") {
                        state.add_bytes_url(
                            DataType::JS,
                            &Url::join(url, a).unwrap_or_else(|_| a.parse().unwrap()),
                        )
                    }
                }
                e => {}
            }
        }
    }
    state.add_bytes(DataType::HTML, string.bytes().len());
    let html = kuchiki::parse_html().one(string);
    for i in try_images_data(&html) {
        state.add_bytes_url(
            DataType::IMG,
            &Url::join(url, &i).unwrap_or_else(|_| i.parse().unwrap()),
        );
    }
    inner(&html, state, url);
}
fn read_html_url(url: &Url, state: &mut CounterState) {
    if let Ok(e) = http_get(url.as_str()) {
        std::fs::write("debug.html", &e).unwrap();
        read_html(&e, url, state);
    }
}

pub fn http_get_len(url: &str) -> Result<usize> {
    Ok(reqwest::blocking::ClientBuilder::new().cookie_store(true).build().unwrap().get(url)
    .header("User-Agent","Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.164 Safari/537.36")
    .header("Accept-Language","fr-FR,fr;q=0.9,en-US;q=0.8,en;q=0.7")
    .header("Accept-Encoding","gzip, deflate")
    .header("Accept","text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8,application/signed-exchange;v=b3;q=0.9")
    .send()?.bytes()?.len())
}

pub fn http_get(url: &str) -> Result<String> {
    Ok(reqwest::blocking::ClientBuilder::new().cookie_store(true).build().unwrap().get(url)
    .header("User-Agent","Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.164 Safari/537.36")
    .header("Accept-Language","fr-FR,fr;q=0.9,en-US;q=0.8,en;q=0.7")
    .header("Accept-Encoding","gzip, deflate")
    .header("Accept","text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8,application/signed-exchange;v=b3;q=0.9")
    .send()?.text()?)
}
