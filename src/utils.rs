use std::{borrow::Cow, collections::HashMap};

use reqwest::Url;

use anyhow::*;

use crate::{
    text_element::{Header, TextCompound},
    text_parser::Context,
};

pub fn filter_names(string: &str) -> &str {
    if string.contains("img") {
        "img"
    } else if string.contains("source") {
        "source"
    } else {
        string
    }
}

const IMAGE_EXTENSIONS: &[&str] = &[
    ".jpg", ".jpeg", ".webm", ".tiff", ".png", ".bmp", ".gif", ".svg",
];
pub fn get_img_link_map<'a>(
    url: &Context,
    attrs: &'a HashMap<String, String>,
) -> Option<Cow<'a, str>> {
    for (a, value) in attrs {
        if !matches!(a.as_str(), "alt" | "class" | "size" | "width" | "height")
            && (value.contains('/') || IMAGE_EXTENSIONS.iter().any(|x| value.ends_with(x)))
        {
            if let Some(e) = get_or_join(&url.url, value) {
                if let Some(k) = &url.meta.image {
                    if k == e.as_ref() {
                        return None;
                    }
                }
                return Some(e);
            }
        }
    }
    None
}

const TEMPLATE: &str = include_str!("../template/template.html");

pub fn gen_html_2(parts: &[TextCompound], ctx: &Context) -> String {
    let k = &[
        TextCompound::Quote(box TextCompound::Link(
            Box::new(TextCompound::Raw(Cow::Borrowed("Official website"))),
            Cow::Borrowed(ctx.url.as_str()),
        )),
        TextCompound::H(
            vec![Cow::Borrowed("main-title")],
            Header::H1,
            box TextCompound::Raw(
                ctx.meta
                    .title
                    .as_ref()
                    .map(|x| Cow::Borrowed(x.as_str()))
                    .unwrap_or_else(|| Cow::Borrowed("")),
            ),
        ),
        TextCompound::Img(
            ctx.meta
                .image
                .as_ref()
                .map(|x| Cow::Borrowed(x.as_str()))
                .unwrap_or_else(|| Cow::Borrowed("")),
        ),
    ]
    .iter()
    .chain(parts.iter())
    .flat_map(|x| x.html(ctx))
    .collect::<Vec<_>>()
    .join("\n");
    if k.contains("<code>") {
        TEMPLATE
            .replace("%%start:code%%", "")
            .replace("%%end%%", "")
            .replace("%%CODE%%", k)
            .replace("%%DOWNLOAD%%", &if ctx.download {
                String::new()
            } else {
                format!("<quote><a href=\"/d/{}\" download=\"article.html\">Download this article</a></quote>",&ctx.min_id)})
    } else {
        use regex::Regex;
        let re = Regex::new(r"%%start:code%%[^%]+%%end%%").unwrap();
        re.replace_all(TEMPLATE, "")
            .replace("%%CODE%%", k)
            .replace("%%DOWNLOAD%%", &if ctx.download {
                String::new()
            } else {
                format!("<quote><a href=\"/d/{}\" download=\"article.html\">Download this article</a></quote>",&ctx.min_id)})
    }
}

pub fn get_or_join<'a>(url: &Url, string: &'a str) -> Option<Cow<'a, str>> {
    let string = if string.contains(' ') {
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
        Some(Cow::Borrowed(string))
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

const ORHER_THAN_HTML: &[&str] = &[
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
