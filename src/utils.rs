use std::borrow::Cow;

use kuchiki::{Attributes, NodeRef};
use reqwest::Url;

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
            return Some(e);
        }
    }
    None
}

pub fn gen_html(parts: &[Part<'_>], ctx: &Context, out_file: &str) {
    let template = std::fs::read_to_string("template.html").unwrap();
    let template = template.replace(
        "%%CODE%%",
        &[
            Part::H(
                Header::H1,
                TextCompound::Raw(Cow::Owned(ctx.meta.title.clone().unwrap_or_default())),
            ),
            Part::Img(Cow::Owned(ctx.meta.image.clone().unwrap_or_default())),
        ]
        .iter()
        .chain(parts.iter())
        .flat_map(|x| x.html())
        .collect::<Vec<_>>()
        .join("\n"),
    );

    std::fs::write(out_file, template).unwrap();
}

pub fn gen_md(parts: &[Part<'_>], ctx: &Context, out_file: &str) {
    let template = [
        Part::H(
            Header::H1,
            TextCompound::Raw(Cow::Owned(ctx.meta.title.clone().unwrap_or_default())),
        ),
        Part::Img(Cow::Owned(ctx.meta.image.clone().unwrap_or_default())),
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

pub fn http_get(url: &str) -> String {
    reqwest::blocking::Client::new().get(url)
    .header("User-Agent","Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.164 Safari/537.36")
    .header("Accept-Language","fr-FR,fr;q=0.9,en-US;q=0.8,en;q=0.7")
    .header("Accept-Encoding","gzip, deflate, br")
    .header("Accept","text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8,application/signed-exchange;v=b3;q=0.9")
    .send().unwrap().text().unwrap()
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
