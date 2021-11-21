use crate::{
    cache::get_shortened_from_url,
    text_element::{Header, TextCompound},
    text_parser::Context,
    utils::is_html,
};

impl<'a> TextCompound<'a> {
    pub fn html(&'a self, ctx: &Context, string: &mut String) {
        match self {
            TextCompound::Raw(a) => {
                string.push_str(&html_escape::encode_text(a));
                string.push(' ');
            }
            TextCompound::Link(a, b) => {
                push_html(
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
                );
            }
            TextCompound::Italic(a) => {
                push_simple_html(string, "i", a, ctx);
            }
            TextCompound::Bold(a) => {
                push_simple_html(string, "b", a, ctx);
            }
            TextCompound::Underline(a) => {
                push_simple_html(string, "u", a, ctx);
            }
            TextCompound::Array(a) => a.iter().for_each(|x| x.html(ctx, string)),
            TextCompound::Abbr(a, b) => {
                push_html(string, "small", Some(("title", b.as_ref())), a, ctx);
            }
            TextCompound::Sup(a) => {
                push_simple_html(string, "sup", a, ctx);
            }
            TextCompound::Sub(a) => {
                push_simple_html(string, "sub", a, ctx);
            }
            TextCompound::Small(a) => {
                push_simple_html(string, "small", a, ctx);
            }
            TextCompound::Br => {
                string.push_str("<br/>");
            }
            TextCompound::Code(a) => {
                if a.contains('\n') {
                    push_simple(string, "pre", |string| {
                        push_simple(string, "code", |string| {
                            string.push_str(&html_escape::encode_text(a));
                        });
                    });
                } else {
                    push_simple(string, "code", |string| {
                        string.push_str(&html_escape::encode_text(a));
                        string.push_str("&nbsp;")
                    });
                }
            }

            TextCompound::Img(a) => {
                string.push_str("<img src=\"");
                string.push_str(a);
                string.push_str("\">");
            }
            Self::H(c, a, b) => {
                let c: Vec<String> = c
                    .iter()
                    .flat_map(|x| ctx.map.get(x.as_ref()))
                    .map(|x| format!("#{}", x))
                    .collect();
                let header = match a {
                    Header::H1 => "h1",
                    Header::H2 => "h2",
                    Header::H3 => "h3",
                    Header::H4 => "h4",
                    Header::H5 => "h5",
                };

                push_html(
                    string,
                    header,
                    if !c.is_empty() {
                        Some(("id".to_string(), c.join(" ")))
                    } else {
                        None
                    },
                    b,
                    ctx,
                );
            }
            Self::Ul(a) => {
                push_simple(string, "il", |string| {
                    for x in a {
                        push_simple_html(string, "li", x, ctx);
                    }
                });
            }

            Self::P(a) => {
                push_simple_html(string, "p", a, ctx);
            }
            Self::Table(a) => {
                push_simple(string, "table", |string| {
                    for i in a {
                        push_simple(string, "tr", |string| {
                            for (a, b) in i {
                                push_simple_html(string, if *a { "th" } else { "td" }, b, ctx);
                            }
                        });
                    }
                });
            }
            Self::Quote(a) => {
                push_simple_html(string, "quote", a, ctx);
            }
        }
    }
}

fn push_simple_html(string: &mut String, a: &str, html: &TextCompound, ctx: &Context) {
    push_html::<String>(string, a, None, html, ctx);
}

fn push_simple(string: &mut String, a: &str, f: impl FnOnce(&mut String)) {
    push(string, a, None as Option<(String, String)>, f);
}

fn push<T: Into<String>>(
    string: &mut String,
    a: &str,
    attribute: Option<(T, T)>,
    f: impl FnOnce(&mut String),
) {
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
    f(string);
    string.push_str("</");
    string.push_str(a);
    string.push('>');
}

fn push_html<T: Into<String>>(
    string: &mut String,
    a: &str,
    attribute: Option<(T, T)>,
    html: &TextCompound,
    ctx: &Context,
) {
    push(string, a, attribute, |string| html.html(ctx, string));
}
