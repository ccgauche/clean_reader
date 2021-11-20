use std::{borrow::Cow, collections::HashMap};

use reqwest::Url;

use anyhow::*;

use crate::{
    text_element::{Header, TextCompound},
    text_parser::Context,
};

const NAMES_TO_FILTER: &[&str] = &["img", "source"];

pub fn filter_names(string: &str) -> &str {
    *NAMES_TO_FILTER
        .iter()
        .find(|x| string.contains(*x))
        .unwrap_or(&string)
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
                return if url
                    .meta
                    .image
                    .as_ref()
                    .map(|x| x == e.as_ref())
                    .unwrap_or(false)
                {
                    None
                } else {
                    Some(e)
                };
            }
        }
    }
    None
}

const TEMPLATE: &str = include_str!("../template/template.html");

pub fn gen_html_2(parts: &[TextCompound], ctx: &Context) -> String {
    let mut string = String::new();
    [
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
    .for_each(|x| x.html(ctx, &mut string));
    if string.contains("<code>") {
        TEMPLATE
        .replace("%%URL%%", ctx.url.as_str())
            .replace("%%start:code%%", "")
            .replace("%%end%%", "")
            .replace("%%CODE%%", &string)
            .replace("%%DOWNLOAD%%", &if ctx.download {
                String::new()
            } else {
                format!("<quote><a href=\"/d/{}\" download=\"article.html\">Download this article</a></quote>",&ctx.min_id)})
    } else {
        use regex::Regex;
        let re = Regex::new(r"%%start:code%%[^%]+%%end%%").unwrap();
        re.replace_all(TEMPLATE, "")
        .replace("%%URL%%", ctx.url.as_str())
            .replace("%%CODE%%", &string)
            .replace("%%DOWNLOAD%%", &if ctx.download {
                String::new()
            } else {
                format!("<quote><a href=\"/d/{}\" download=\"article.html\">Download this article</a></quote>",&ctx.min_id)})
    }
}

fn before(string: &str, c: char) -> &str {
    &string[..string.find(c).unwrap_or_else(|| string.len())]
}

pub fn get_or_join<'a>(url: &Url, string: &'a str) -> Option<Cow<'a, str>> {
    let string = if string.contains(' ') {
        before(before(string, ',').trim(), ' ')
    } else {
        string
    };
    if string.starts_with("data") {
        None
    } else if string.starts_with("http") {
        Some(Cow::Borrowed(string))
    } else {
        Some(Cow::Owned(url.join(string).ok()?.to_string()))
    }
}

fn latin1_to_string(s: &[u8]) -> String {
    s.iter().map(|&c| c as char).collect()
}

pub fn http_get(url: &str) -> Result<String> {
    let k = reqwest::blocking::ClientBuilder::new().cookie_store(true).build().unwrap().get(url)
    .header("User-Agent","Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.164 Safari/537.36")
    .header("Accept-Language","fr-FR,fr;q=0.9,en-US;q=0.8,en;q=0.7")
    .header("Accept-Encoding","gzip, deflate")
    .header("Accept","text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8,application/signed-exchange;v=b3;q=0.9")
    .send()?.bytes()?;
    if k.len() > 1024 * 1024 {
        return Err(anyhow!("File too big"));
    }
    let k1 = String::from_utf8_lossy(&k);
    if k1.contains("<meta charset=\"iso-8859-1\">") {
        Ok(latin1_to_string(&k))
    } else {
        Ok(k1.to_string())
    }
}

const ORHER_THAN_HTML: &[&str] = &[
    ".png", ".jpg", ".jpeg", ".gif", ".pdf", ".doc", ".css", ".js", ".bmp", ".webm", ".mp4",
    ".mp3", ".mov", ".tiff", ".tif", ".zip", ".tar", ".gz", ".7z", ".rar", ".py", ".rs", ".c",
    ".xls", ".odt", ".ods", ".wav", ".flac", ".avi", ".m4a", ".json", ".ico", ".ttf", ".woff",
    ".woff2",
];

pub fn is_html(url: &str) -> bool {
    !ORHER_THAN_HTML.iter().any(|x| url.ends_with(x))
}

pub fn sha256(data: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(data.as_bytes());
    hex::encode(hasher.finalize())
}
