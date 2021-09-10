use std::borrow::Cow;

use crate::{cache::get_shortened_from_url, utils::is_text};

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
    Code(Cow<'a, str>),
    Img(Cow<'a, str>),
    Br,
}

pub trait Compilable {
    fn markdown<'a>(&'a self) -> Option<Cow<'a, str>>;
    fn html<'a>(&'a self) -> Option<Cow<'a, str>>;
}

impl Compilable for TextCompound<'_> {
    fn markdown<'a>(&'a self) -> Option<Cow<'a, str>> {
        let k = match self {
            TextCompound::Raw(a) => a.clone(),
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
        };
        if k.trim().is_empty() && !matches!(self, TextCompound::Br) {
            return None;
        }
        Some(k)
    }
    fn html<'a>(&'a self) -> Option<Cow<'a, str>> {
        let k: Cow<'a, str> = match self {
            TextCompound::Raw(a) => Cow::Owned(format!("{} ", html_escape::encode_text(a))),
            TextCompound::Link(a, b) => {
                let a = a.html()?;
                if is_text(b.as_ref()) && !a.contains("Official website") {
                    Cow::Owned(format!(
                        "<a href=\"/m/{}\">{}</a>",
                        get_shortened_from_url(b),
                        a
                    ))
                } else {
                    Cow::Owned(format!("<a href=\"{}\">{}</a>", b, a))
                }
            }
            TextCompound::Italic(a) => Cow::Owned(format!("<i>{} </i>", a.html()?)),
            TextCompound::Bold(a) => Cow::Owned(format!("<b>{} </b>", a.html()?)),
            TextCompound::Array(a) => {
                Cow::Owned(a.iter().flat_map(|x| x.html()).collect::<Vec<_>>().join(""))
            }
            TextCompound::Abbr(a, b) => {
                Cow::Owned(format!("<abbr title=\"{}\">{} </abbr>", b, a.html()?))
            }
            TextCompound::Sup(a) => Cow::Owned(format!("<sup>{} </sup>", a.html()?)),
            TextCompound::Sub(a) => Cow::Owned(format!("<sub>{} </sub>", a.html()?)),
            TextCompound::Small(a) => Cow::Owned(format!("<small>{} </small>", a.html()?)),
            TextCompound::Br => Cow::Owned("<br/>".to_owned()),
            TextCompound::Code(a) => {
                if a.contains("\n") {
                    Cow::Owned(format!(
                        "<pre><code>{}</code></pre>",
                        html_escape::encode_text(a)
                    ))
                } else {
                    Cow::Owned(format!(
                        "<code>{}&nbsp;</code>",
                        html_escape::encode_text(a)
                    ))
                }
            }
            TextCompound::Img(a) => Cow::Owned(format!("<img src=\"{}\">", a)),
        };
        if k.trim().is_empty() {
            return None;
        }
        Some(k)
    }
}

#[derive(Debug)]
pub enum Part<'a> {
    H(Header, TextCompound<'a>),
    P(TextCompound<'a>),
    Quote(TextCompound<'a>),
    PlainText(TextCompound<'a>),
    Ul(Vec<TextCompound<'a>>),
    Table(Vec<Vec<(bool, TextCompound<'a>)>>),
}

#[derive(Debug)]
pub enum Header {
    H1,
    H2,
    H3,
    H4,
    H5,
}

impl Compilable for Part<'_> {
    fn markdown<'a>(&'a self) -> Option<Cow<'a, str>> {
        match self {
            Part::H(a, b) => Some(Cow::Owned(format!(
                "{} {}",
                match a {
                    Header::H1 => "#",
                    Header::H2 => "##",
                    Header::H3 => "###",
                    Header::H4 => "####",
                    Header::H5 => "#####",
                },
                b.markdown()?
            ))),
            Part::Ul(a) => Some(Cow::Owned(
                a.iter()
                    .flat_map(|x| Some(format!(" - {}", x.markdown()?)))
                    .collect::<Vec<_>>()
                    .join(""),
            )),
            Part::P(a) => a.markdown(),
            Part::PlainText(a) => a.markdown(),
            Part::Table(a) => {
                let mut iter = a.iter();
                let mut construct = String::from("\n");
                if let Some(e) = iter.next() {
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
                    Some(Cow::Owned(construct))
                } else {
                    Some(Cow::Owned(String::new()))
                }
            }
            Part::Quote(a) => Some(Cow::Owned(format!(" > {}", a.markdown()?))),
        }
    }

    fn html<'a>(&'a self) -> Option<Cow<'a, str>> {
        match self {
            Part::H(a, b) => {
                let header = match a {
                    Header::H1 => 1,
                    Header::H2 => 2,
                    Header::H3 => 3,
                    Header::H4 => 4,
                    Header::H5 => 5,
                };
                Some(Cow::Owned(format!(
                    "<h{}>{}</h{}>",
                    header,
                    b.html()?,
                    header
                )))
            }
            Part::Ul(a) => Some(Cow::Owned(format!(
                "<ul>{}</ul>",
                a.iter()
                    .flat_map(|x| Some(format!("<li>{}</li>", x.html()?)))
                    .collect::<Vec<_>>()
                    .join("")
            ))),
            Part::P(a) => Some(Cow::Owned(format!("<p>{}</p>", a.html()?))),
            Part::PlainText(a) => Some(Cow::Owned(format!("{}", &a.html()?))),
            Part::Table(a) => {
                let mut string = String::from("<table>");
                for i in a {
                    string.push_str("<tr>");
                    for (a, b) in i {
                        string.push_str(if *a { "<th>" } else { "<td>" });
                        string.push_str(&b.html()?);
                        string.push_str(if *a { "</th>" } else { "</td>" });
                    }
                    string.push_str("</tr>");
                }
                string.push_str("</table>");
                Some(Cow::Owned(string))
            }
            Part::Quote(a) => Some(Cow::Owned(format!("<quote>{}</quote>", a.html()?))),
        }
    }
}
