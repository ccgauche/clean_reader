use std::{
    borrow::{Borrow, Cow},
    fmt::Display,
};

use crate::{cache::get_shortened_from_url, text_parser::Context, utils::is_text};

#[derive(Debug)]
pub enum TextCompound<'a> {
    Raw(Cow<'a, str>),
    Link(Box<TextCompound<'a>>, Cow<'a, str>),
    Italic(Box<TextCompound<'a>>),
    Bold(Box<TextCompound<'a>>),
    Array(Vec<TextCompound<'a>>),
    Abbr(Box<TextCompound<'a>>, Cow<'a, str>),
    Sup(Box<TextCompound<'a>>),
    Sub(Box<TextCompound<'a>>),
    Small(Box<TextCompound<'a>>),
    Code(String),
    Img(Cow<'a, str>),
    Br,
    H(Vec<Cow<'a, str>>, Header, Box<TextCompound<'a>>),
    P(Box<TextCompound<'a>>),
    Quote(Box<TextCompound<'a>>),
    Ul(Vec<TextCompound<'a>>),
    Table(Vec<Vec<(bool, TextCompound<'a>)>>),
}
impl Display for TextCompound<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TextCompound::Raw(a) => write!(f, "Raw({})", a),
            TextCompound::Link(a, b) => write!(f, "Raw({},{})", a, b),
            TextCompound::Italic(a) => write!(f, "Italic({})", a),
            TextCompound::Bold(a) => write!(f, "Bold({})", a),
            TextCompound::Array(a) => write!(
                f,
                "{}",
                a.iter()
                    .map(|x| x.to_string())
                    .collect::<Vec<_>>()
                    .join("+")
            ),
            TextCompound::Abbr(a, b) => write!(f, "Abbr({},{})", a.to_string(), b),
            TextCompound::Sup(a) => write!(f, "Sup({})", a),
            TextCompound::Sub(a) => write!(f, "Sub({})", a),
            TextCompound::Small(a) => write!(f, "Small({})", a),
            TextCompound::Code(a) => write!(f, "Code({})", a),
            TextCompound::Img(a) => write!(f, "Img({})", a),
            TextCompound::Br => write!(f, "Br()"),
            TextCompound::H(a, b, c) => write!(
                f,
                "H{}([{}],{})",
                match b {
                    Header::H1 => "1",
                    Header::H2 => "2",
                    Header::H3 => "3",
                    Header::H4 => "4",
                    Header::H5 => "5",
                },
                a.iter()
                    .map(|x| x.to_string())
                    .collect::<Vec<_>>()
                    .join(","),
                c
            ),
            TextCompound::P(a) => write!(f, "P({})", a),
            TextCompound::Quote(a) => write!(f, "Quote({})", a),
            TextCompound::Ul(a) => write!(
                f,
                "Ul({})",
                a.iter()
                    .map(|x| x.to_string())
                    .collect::<Vec<_>>()
                    .join(",")
            ),
            TextCompound::Table(a) => write!(
                f,
                "Table([{}])",
                a.iter()
                    .map(|x| x
                        .iter()
                        .map(|x| x.1.to_string())
                        .collect::<Vec<_>>()
                        .join(","))
                    .collect::<Vec<_>>()
                    .join("],[")
            ),
        }
    }
}
impl<'a> TextCompound<'a> {
    pub fn remove_title(self) -> Self {
        match self {
            Self::Array(mut e) => {
                if matches!(e.get(0), Some(Self::H(_, Header::H1, _))) {
                    e.remove(0);
                }
                Self::Array(e)
            }
            e => e,
        }
    }

    #[allow(unused)]
    pub fn markdown(&'a self) -> Option<Cow<'a, str>> {
        let k = match self {
            TextCompound::Raw(a) => Cow::Borrowed(a.borrow()),
            TextCompound::Link(a, b) => Cow::Owned(format!("[{}]({})", a.markdown()?, b)),
            TextCompound::Italic(a) => Cow::Owned(format!("*{}*", a.markdown()?)),
            TextCompound::Bold(a) => Cow::Owned(format!("**{}**", a.markdown()?)),
            TextCompound::Array(a) => Cow::Owned(
                a.iter()
                    .flat_map(|x| x.markdown())
                    .collect::<Vec<_>>()
                    .join(""),
            ),
            TextCompound::Abbr(a, b) => Cow::Owned(format!("{} (*{}*)", a.markdown()?, b)),
            TextCompound::Sup(a) => Cow::Owned(format!("^{}^", a.markdown()?)),
            TextCompound::Sub(a) => Cow::Owned(format!("~{}~", a.markdown()?)),
            TextCompound::Small(a) => Cow::Owned(format!("~^{}^~", a.markdown()?)),
            TextCompound::Br => Cow::Borrowed("\n"),
            TextCompound::Code(a) => Cow::Owned(format!("\n```\n{}\n```\n", a)),
            TextCompound::Img(a) => Cow::Owned(format!("![{}]({})", a, a)),
            Self::H(_, a, b) => Cow::Owned(format!(
                "{} {}",
                match a {
                    Header::H1 => "#",
                    Header::H2 => "##",
                    Header::H3 => "###",
                    Header::H4 => "####",
                    Header::H5 => "#####",
                },
                b.markdown()?
            )),

            Self::Ul(a) => Cow::Owned(
                a.iter()
                    .flat_map(|x| Some(format!(" - {}", x.markdown()?)))
                    .collect::<Vec<_>>()
                    .join(""),
            ),
            Self::P(a) => a.markdown()?,
            Self::Table(a) => {
                let mut iter = a.iter();
                let mut construct = String::from("\n");
                Cow::Owned(if let Some(e) = iter.next() {
                    construct.push_str(&format!(
                        "\n|{}|\n",
                        e.iter()
                            .flat_map(|(_, x)| x.markdown())
                            .collect::<Vec<_>>()
                            .join("|")
                    ));
                    construct.push_str(&format!(
                        "|{}|\n",
                        e.iter().map(|_| "---").collect::<Vec<_>>().join("|")
                    ));
                    for e in iter {
                        construct.push_str(&format!(
                            "|{}|\n",
                            e.iter()
                                .flat_map(|(_, x)| x.markdown())
                                .collect::<Vec<_>>()
                                .join("|")
                        ));
                    }
                    construct
                } else {
                    String::new()
                })
            }
            Self::Quote(a) => Cow::Owned(format!(" > {}", a.markdown()?)),
        };
        if k.trim().is_empty() && !matches!(self, TextCompound::Br) {
            return None;
        }
        Some(k)
    }
    pub fn html(&'a self, ctx: &Context) -> Option<Cow<'a, str>> {
        let k: Cow<'a, str> = match self {
            TextCompound::Raw(a) => html_escape::encode_text(a),
            TextCompound::Link(a, b) => {
                let a = a.html(ctx)?;
                Cow::Owned(
                    if !ctx.download
                        && (b.starts_with('#')
                            && is_text(b.as_ref())
                            && !a.contains("Official website"))
                    {
                        format!("<a href=\"/m/{}\">{}</a>", get_shortened_from_url(b), a)
                    } else {
                        format!("<a href=\"{}\">{}</a>", b, a)
                    },
                )
            }
            TextCompound::Italic(a) => Cow::Owned(format!("<i>{} </i>", a.html(ctx)?)),
            TextCompound::Bold(a) => Cow::Owned(format!("<b>{} </b>", a.html(ctx)?)),
            TextCompound::Array(a) => Cow::Owned(
                a.iter()
                    .flat_map(|x| x.html(ctx))
                    .collect::<Vec<_>>()
                    .join(""),
            ),
            TextCompound::Abbr(a, b) => {
                Cow::Owned(format!("<abbr title=\"{}\">{} </abbr>", b, a.html(ctx)?))
            }
            TextCompound::Sup(a) => Cow::Owned(format!("<sup>{} </sup>", a.html(ctx)?)),
            TextCompound::Sub(a) => Cow::Owned(format!("<sub>{} </sub>", a.html(ctx)?)),
            TextCompound::Small(a) => Cow::Owned(format!("<small>{} </small>", a.html(ctx)?)),
            TextCompound::Br => Cow::Borrowed("<br/>"),
            TextCompound::Code(a) => Cow::Owned(if a.contains('\n') {
                format!("<pre><code>{}</code></pre>", html_escape::encode_text(a))
            } else {
                format!("<code>{}&nbsp;</code>", html_escape::encode_text(a))
            }),
            TextCompound::Img(a) => Cow::Owned(format!("<img src=\"{}\">", a)),
            Self::H(c, a, b) => {
                let c: Vec<String> = c
                    .iter()
                    .flat_map(|x| ctx.map.get(x.as_ref()))
                    .map(|x| format!("#{}", x))
                    .collect();
                let header = match a {
                    Header::H1 => 1,
                    Header::H2 => 2,
                    Header::H3 => 3,
                    Header::H4 => 4,
                    Header::H5 => 5,
                };
                Cow::Owned(format!(
                    "<h{}{}>{}</h{}>",
                    header,
                    if c.is_empty() {
                        String::new()
                    } else {
                        format!(" id=\"{}\"", c.join(" "))
                    },
                    b.html(ctx)?,
                    header
                ))
            }
            Self::Ul(a) => Cow::Owned(format!(
                "<ul>{}</ul>",
                a.iter()
                    .flat_map(|x| Some(format!("<li>{}</li>", x.html(ctx)?)))
                    .collect::<Vec<_>>()
                    .join("")
            )),

            Self::P(a) => Cow::Owned(format!("<p>{}</p>", a.html(ctx)?)),
            Self::Table(a) => {
                let mut string = String::from("<table>");
                for i in a {
                    string.push_str("<tr>");
                    for (a, b) in i {
                        string.push_str(if *a { "<th>" } else { "<td>" });
                        string.push_str(&b.html(ctx)?);
                        string.push_str(if *a { "</th>" } else { "</td>" });
                    }
                    string.push_str("</tr>");
                }
                string.push_str("</table>");
                Cow::Owned(string)
            }
            Self::Quote(a) => Cow::Owned(format!("<quote>{}</quote>", a.html(ctx)?)),
        };
        if k.trim().is_empty() {
            return None;
        }
        Some(k)
    }
}

#[derive(Debug)]
pub enum Header {
    H1,
    H2,
    H3,
    H4,
    H5,
}
