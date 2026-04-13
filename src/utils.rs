use std::{borrow::Cow, collections::HashMap};

use once_cell::sync::Lazy;
use reqwest::{
    header::{self, HeaderMap, HeaderValue},
    Url,
};

use crate::{
    config::CONFIG,
    error::{Error, Result},
    text_element::{Header, TextCompound},
    text_parser::Context,
};

const NAMES_TO_FILTER: &[&str] = &["img", "source"];

pub fn filter_names(string: &str) -> &str {
    NAMES_TO_FILTER
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
                return url
                    .meta
                    .image
                    .as_ref()
                    .map(|x| x != e.as_ref())
                    .unwrap_or(true)
                    .then_some(e);
            }
        }
    }
    None
}

fn before(string: &str, c: char) -> &str {
    &string[..string.find(c).unwrap_or(string.len())]
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
    s.iter().copied().map(char::from).collect()
}

fn default_headers() -> HeaderMap {
    let mut h = HeaderMap::new();
    h.insert(
        header::USER_AGENT,
        HeaderValue::from_static(
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.164 Safari/537.36",
        ),
    );
    h.insert(
        header::ACCEPT_LANGUAGE,
        HeaderValue::from_static("fr-FR,fr;q=0.9,en-US;q=0.8,en;q=0.7"),
    );
    h.insert(
        header::ACCEPT_ENCODING,
        HeaderValue::from_static("gzip, deflate"),
    );
    h.insert(
        header::ACCEPT,
        HeaderValue::from_static(
            "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8,application/signed-exchange;v=b3;q=0.9",
        ),
    );
    h
}

static ASYNC_CLIENT: Lazy<reqwest::Client> = Lazy::new(|| {
    reqwest::Client::builder()
        .cookie_store(true)
        .default_headers(default_headers())
        .build()
        .expect("failed to build async reqwest client")
});

static BLOCKING_CLIENT: Lazy<reqwest::blocking::Client> = Lazy::new(|| {
    reqwest::blocking::Client::builder()
        .cookie_store(true)
        .default_headers(default_headers())
        .build()
        .expect("failed to build blocking reqwest client")
});

const IMAGE_SIZE_LIMIT: u64 = 50_000_000;

/// Blocking image fetch — called from the std::thread image worker.
pub fn http_get_bytes(url: &str) -> Result<Vec<u8>> {
    let resp = BLOCKING_CLIENT.get(url).send()?;
    if resp.content_length().unwrap_or(0) > IMAGE_SIZE_LIMIT {
        return Err(Error::ResponseTooLarge {
            limit: IMAGE_SIZE_LIMIT,
        });
    }
    let bytes = resp.bytes()?;
    if bytes.len() as u64 > IMAGE_SIZE_LIMIT {
        return Err(Error::ResponseTooLarge {
            limit: IMAGE_SIZE_LIMIT,
        });
    }
    Ok(bytes.to_vec())
}

/// Async article fetch — used by the main pipeline.
pub async fn http_get(url: &str) -> Result<String> {
    let resp = ASYNC_CLIENT.get(url).send().await?;
    if resp.content_length().unwrap_or(0) > CONFIG.max_size {
        return Err(Error::ResponseTooLarge {
            limit: CONFIG.max_size,
        });
    }
    let bytes = resp.bytes().await?;
    if bytes.len() as u64 > CONFIG.max_size {
        return Err(Error::ResponseTooLarge {
            limit: CONFIG.max_size,
        });
    }
    let lossy = String::from_utf8_lossy(&bytes);
    let head = &lossy[0..lossy.find("</head>").unwrap_or(lossy.len())];
    if head.contains("iso-8859-1") {
        Ok(latin1_to_string(&bytes))
    } else {
        Ok(lossy.into_owned())
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

#[derive(askama::Template)]
#[template(path = "article.html", escape = "html")]
struct ArticleTemplate<'a> {
    url: &'a str,
    code: &'a str,
    has_code: bool,
    download_link: Option<String>,
}

// Justification: we want to launch all image compression before joining the threads
#[allow(clippy::needless_collect)]
pub fn gen_html_2(parts: &[TextCompound], ctx: &mut Context) -> String {
    use askama::Template;

    let ctx1 = ctx.clone();
    let mut body = String::with_capacity(50_000);
    let joinlist = [
        TextCompound::H(
            vec![Cow::Borrowed("main-title")],
            Header::H1,
            Box::new(TextCompound::Raw(
                ctx1.meta
                    .title
                    .as_ref()
                    .map(|x| Cow::Borrowed(x.as_str()))
                    .unwrap_or_else(|| Cow::Borrowed("")),
            )),
        ),
        TextCompound::Img(
            ctx1.meta
                .image
                .as_ref()
                .map(|x| Cow::Borrowed(x.as_str()))
                .unwrap_or_else(|| Cow::Borrowed("")),
        ),
    ]
    .iter()
    .chain(parts.iter())
    .flat_map(|x| x.html(ctx, &mut body))
    .collect::<Vec<_>>();

    joinlist.into_iter().for_each(|x| {
        x.join().unwrap();
    });

    let download_link = (!ctx.download).then(|| format!("/d/{}", ctx.min_id));
    let has_code = body.contains("<code>");
    ArticleTemplate {
        url: ctx.url.as_str(),
        code: &body,
        has_code,
        download_link,
    }
    .render()
    .expect("article template render failed")
}
