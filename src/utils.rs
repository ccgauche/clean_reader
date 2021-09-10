use std::borrow::Cow;

use kuchiki::{Attributes, NodeRef};
use reqwest::Url;

use anyhow::*;

use crate::{
    structures::{Compilable, Header, Part, TextCompound},
    text_parser::Context,
};

const blacklists: &'static [(&'static [&'static str], usize)] = &[
    (&["abonn", "rserv"], 100),
    (&["lireaussi"], 100),
    (&["partage"], 200),
    (&["newsletter"], 200),
    (&["notif"], 200),
    (&["commentaire"], 30),
    (&["inscris", "compte"], 100),
    (&["lespluslus"], 300),
    (&["accueil", "abonns"], 500),
];

pub fn valid_text(text: &str, title: &Context, element: &str) -> bool {
    if element == "code" || element == "div" {
        return true;
    }
    let p = text
        .to_lowercase()
        .chars()
        .filter(|x| match x {
            'a'..='z' | '\'' => true,

            _ => false,
        })
        .collect::<String>();
    'a: for (a, b) in blacklists {
        if p.len() > *b {
            continue;
        }
        for a in *a {
            if !p.contains(a) {
                continue 'a;
            }
        }
        return false;
    }

    if let Some(e) = &title.meta.title {
        let p1 = e
            .to_lowercase()
            .chars()
            .filter(|x| match x {
                'a'..='z' | '\'' => true,

                _ => false,
            })
            .collect::<String>();
        if p == p1 {
            return false;
        }
    }
    if element.starts_with("h") && element.len() == 2 {
        return p.len() > 3;
    }

    p.len() > 10
}

const TO_SEARCH: &'static [&'static str] = &[
    "data-src-large",
    "data-echo",
    "data-original",
    "data-src",
    "src",
    "srcset",
    "data-li-src",
];

pub fn get_img_link(url: &Context, attrs: &Attributes) -> Option<Cow<'static, str>> {
    println!("{:?}", attrs);

    for i in TO_SEARCH {
        if let Some(e) = attrs
            .get(*i)
            .map(|x| get_or_join(&url.url, x, *i == "data-src"))
            .flatten()
        {
            if let Some(k) = &url.meta.image {
                if k == e.as_ref() {
                    return None;
                }
            }
            return Some(e);
        }
    }
    None
}

const TEMPLATE: &'static str = include_str!("../template.html");

pub fn gen_html(parts: &[Part<'_>], ctx: &Context) -> String {
    println!("SSR {}",ctx.url.as_str());
    TEMPLATE.replace(
        "%%CODE%%",
        &[
            Part::Quote(TextCompound::Link(Box::new(TextCompound::Raw(Cow::Owned("Official website".to_owned()))), Cow::Owned(ctx.url.as_str().to_owned()))),
            Part::H(
                Header::H1,
                TextCompound::Raw(Cow::Owned(ctx.meta.title.clone().unwrap_or_default())),
            ),
            Part::PlainText(TextCompound::Img(Cow::Owned(ctx.meta.image.clone().unwrap_or_default()))),
        ]
        .iter()
        .chain(parts.iter())
        .flat_map(|x| x.html())
        .collect::<Vec<_>>()
        .join("\n"),
    )
}

pub fn gen_md(parts: &[Part<'_>], ctx: &Context, out_file: &str) {
    let template = [
        Part::H(
            Header::H1,
            TextCompound::Raw(Cow::Owned(ctx.meta.title.clone().unwrap_or_default())),
        ),
        Part::PlainText(TextCompound::Img(Cow::Owned(ctx.meta.image.clone().unwrap_or_default()))),
    ]
    .iter()
    .chain(parts.iter())
    .flat_map(|x| x.markdown())
    .collect::<Vec<_>>()
    .join("\n");

    std::fs::write(out_file, template).unwrap();
}

pub fn get_or_join(url: &Url, string: &str, is_srcset: bool) -> Option<Cow<'static, str>> {
    let string = if is_srcset {
        string
            .split(",")
            .next()
            .unwrap()
            .trim()
            .split(" ")
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

const ORHER_THAN_HTML: &'static [&'static str] = &[
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

pub fn remove(on: &NodeRef, selector: &str) {
    loop {
        let i = on.select(selector).unwrap().next();
        if let Some(e) = i {
            e.as_node().detach();
        } else {
            break;
        }
    }
}

pub fn sha256(data: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(data.as_bytes());
    hex::encode(hasher.finalize())
}
