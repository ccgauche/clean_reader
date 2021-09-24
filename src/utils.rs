use std::{borrow::Cow, collections::HashMap};

use reqwest::Url;

use anyhow::*;

use crate::{new_arch::text_element,  text_parser::Context};

pub fn get_img_link_map(url: &Context, attrs: &HashMap<String,String>) -> Option<Cow<'static, str>> {
    for (a,value) in attrs {
        if !matches!(a.as_str(), "alt" | "class" | "size" | "width" | "height") {
            if value.contains("/") || value.ends_with(".jpg") || value.ends_with(".jpeg")
            || value.ends_with(".webm")|| value.ends_with(".tiff")|| value.ends_with(".png")|| value.ends_with(".bmp")
            || value.ends_with(".gif")|| value.ends_with(".svg") {
                if let Some(e) = get_or_join(&url.url, value)
        {
            if let Some(k) = &url.meta.image {
                if k == e.as_ref() {
                    return None;
                }
            }
            return Some(e);
        }
            }
        }
    }
    None
}

const TEMPLATE: &str = include_str!("../template.html");

pub fn gen_html_2(parts: &[text_element::TextCompound], ctx: &Context) -> String {
    let k = 
        &[
            text_element::TextCompound::Quote(box text_element::TextCompound::Link(Box::new(text_element::TextCompound::Raw("Official website".to_owned())), ctx.url.as_str().to_owned())),
            text_element::TextCompound::H(
                vec!["main-title".to_owned()],
                text_element::Header::H1,
                box text_element::TextCompound::Raw(ctx.meta.title.clone().unwrap_or_default()),
            ),
            text_element::TextCompound::Img(ctx.meta.image.clone().unwrap_or_default()),
        ]
        .iter()
        .chain(parts.iter())
        .flat_map(|x| x.html(ctx))
        .collect::<Vec<_>>()
        .join("\n");
    if k.contains("<code>") {
        TEMPLATE.replace("%%start:code%%", "").replace("%%end%%","").replace(
            "%%CODE%%",
            k,
        )
    } else {
        use regex::Regex;
        let re = Regex::new(r"%%start:code%%[^%]+%%end%%").unwrap();
        re.replace_all(TEMPLATE, "")
    .replace(
        "%%CODE%%",
        k,
    )
    }
}

pub fn get_or_join(url: &Url, string: &str) -> Option<Cow<'static, str>> {
    let string = if string.contains(" ") {
        string
            .split(',')
            .next()
            .unwrap()
            .trim()
            .split(' ')
            .next()
            .unwrap()
    } else {
        string
    };
    if string.starts_with("data") {
        return None;
    }
    if string.starts_with("http") {
        Some(Cow::Owned(string.to_owned()))
    } else {
        Some(Cow::Owned(url.join(string).ok()?.to_string()))
    }
}

pub fn http_get(url: &str) -> Result<String> {
    Ok(reqwest::blocking::ClientBuilder::new().cookie_store(true).build().unwrap().get(url)
    
    .header("User-Agent","Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.164 Safari/537.36")
    .header("Accept-Language","fr-FR,fr;q=0.9,en-US;q=0.8,en;q=0.7")
    .header("Accept-Encoding","gzip, deflate")
    .header("Accept","text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8,application/signed-exchange;v=b3;q=0.9")
    .send()?.text()?)
}

const ORHER_THAN_HTML: & [&str] = &[
    ".png", ".jpg", ".jpeg", ".gif", ".pdf", ".doc", ".css", ".js", ".bmp", ".webm", ".mp4",
    ".mp3", ".mov", ".tiff", ".tif", ".zip", ".tar", ".gz", ".7z", ".rar", ".py", ".rs", ".c",
    ".xls", ".odt", ".ods", ".wav", ".flac", ".avi", ".m4a", ".json", ".ico", ".ttf", ".woff",
    ".woff2",
];

pub fn is_text(url: &str) -> bool {
    for i in ORHER_THAN_HTML {
        if url.ends_with(i) {
            return false;
        }
    }
    true
}

pub fn sha256(data: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(data.as_bytes());
    hex::encode(hasher.finalize())
}
