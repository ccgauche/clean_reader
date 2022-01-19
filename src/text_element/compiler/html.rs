use std::thread::JoinHandle;

use crate::{
    cache::get_shortened_from_url, image::get_image_url, text_element::TextCompound,
    text_parser::Context, utils::is_html,
};

const PONCTUATION: &str = ".,;:!?()[]{}";

impl<'a> TextCompound<'a> {
    pub fn html(&'a self, ctx: &Context, string: &mut String) -> Vec<JoinHandle<()>> {
        match self {
            Self::Raw(a) => {
                if let Some(e) = a.chars().next() {
                    if PONCTUATION.contains(e) {
                        match string.pop() {
                            Some(' ') => (),
                            Some(e) => string.push(e),
                            _ => (),
                        }
                    }
                }
                string.push_str(&html_escape::encode_text(a));
                vec![]
            }
            Self::Link(a, b) => push_html(
                string,
                "a",
                Some((
                    "href".to_owned(),
                    if !ctx.download
                        && !b.starts_with("mailto:")
                        && !b.starts_with('#')
                        && is_html(b.as_ref())
                    {
                        format!("/m/{}", get_shortened_from_url(b))
                    } else {
                        b.to_string()
                    },
                )),
                a,
                ctx,
            ),
            Self::Italic(a) => push_simple_html(string, "i", a, ctx),
            Self::Bold(a) => push_simple_html(string, "b", a, ctx),
            Self::Underline(a) => push_simple_html(string, "u", a, ctx),
            Self::Array(a) => return a.iter().flat_map(|x| x.html(ctx, string)).collect(),
            Self::Abbr(a, b) => push_html(string, "small", Some(("title", b.as_ref())), a, ctx),
            Self::Sup(a) => push_simple_html(string, "sup", a, ctx),
            Self::Sub(a) => push_simple_html(string, "sub", a, ctx),
            Self::Small(a) => push_simple_html(string, "small", a, ctx),
            Self::Br => {
                string.push_str("<br/>");
                vec![]
            }
            Self::Code(a) => {
                if a.contains('\n') {
                    push_simple(string, "pre", |string| {
                        push_simple(string, "code", |string| {
                            string.push_str(&html_escape::encode_text(a));
                            vec![]
                        })
                    })
                } else {
                    push_simple(string, "code", |string| {
                        string.push_str(&html_escape::encode_text(a));
                        string.push_str("&nbsp;");
                        vec![]
                    })
                }
            }

            Self::Img(a) => {
                string.push_str("<img src=\"");
                let (a, b) = get_image_url(a);
                string.push_str(&a);
                string.push_str("\">");
                if let Some(b) = b {
                    vec![b]
                } else {
                    vec![]
                }
            }
            Self::H(c, a, b) => {
                let c: Vec<String> = c
                    .iter()
                    .flat_map(|x| ctx.map.get(x.as_ref()))
                    .map(|x| format!("#{}", x))
                    .collect();

                push_html(
                    string,
                    a.to_str(),
                    if !c.is_empty() {
                        Some(("id".to_string(), c.join(" ")))
                    } else {
                        None
                    },
                    b,
                    ctx,
                )
            }
            Self::Ul(a) => push_simple(string, "il", |string| {
                a.iter()
                    .flat_map(|x| push_simple_html(string, "li", x, ctx))
                    .collect()
            }),

            Self::P(a) => push_simple_html(string, "p", a, ctx),
            Self::Table(a) => push_simple(string, "table", |string| {
                a.iter()
                    .flat_map(|i| {
                        push_simple(string, "tr", |string| {
                            i.iter()
                                .flat_map(|(a, b)| {
                                    push_simple_html(string, if *a { "th" } else { "td" }, b, ctx)
                                })
                                .collect()
                        })
                    })
                    .collect()
            }),
            Self::Quote(a) => push_simple_html(string, "quote", a, ctx),
        }
    }
}

fn push_simple_html(
    string: &mut String,
    a: &str,
    html: &TextCompound,
    ctx: &Context,
) -> Vec<JoinHandle<()>> {
    push_html::<String>(string, a, None, html, ctx)
}

fn push_simple(
    string: &mut String,
    a: &str,
    f: impl FnOnce(&mut String) -> Vec<JoinHandle<()>>,
) -> Vec<JoinHandle<()>> {
    push(string, a, None as Option<(String, String)>, f)
}

fn push<T: Into<String>, E>(
    string: &mut String,
    a: &str,
    attribute: Option<(T, T)>,
    f: impl FnOnce(&mut String) -> E,
) -> E {
    string.push('<');
    string.push_str(a);
    if let Some((a, b)) = attribute {
        string.push(' ');
        string.push_str(&a.into());
        string.push_str("=\"");
        string.push_str(&b.into());
        string.push('"');
    }
    string.push('>');
    let k = f(string);
    string.push_str("</");
    string.push_str(a);
    string.push_str("> ");
    k
}

fn push_html<T: Into<String>>(
    string: &mut String,
    a: &str,
    attribute: Option<(T, T)>,
    html: &TextCompound,
    ctx: &Context,
) -> Vec<JoinHandle<()>> {
    push(string, a, attribute, |string| html.html(ctx, string))
}
