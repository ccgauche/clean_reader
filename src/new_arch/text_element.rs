use crate::{cache::get_shortened_from_url, text_parser::Context, utils::is_text};

#[derive(Debug)]
pub enum TextCompound {
    Raw(String),
    Link(Box<TextCompound>, String),
    Italic(Box<TextCompound>),
    Bold(Box<TextCompound>),
    Array(Vec<TextCompound>),
    Abbr(Box<TextCompound>, String),
    Sup(Box<TextCompound>),
    Sub(Box<TextCompound>),
    Small(Box<TextCompound>),
    Code(String),
    Img(String),
    Br,
    H(Vec<String>, Header, Box<TextCompound>),
    P(Box<TextCompound>),
    Quote(Box<TextCompound>),
    Ul(Vec<TextCompound>),
    Table(Vec<Vec<(bool, TextCompound)>>),
}
impl TextCompound {
    #[allow(unused)]
    fn markdown(&self) -> Option<String> {
        let k = match self {
            TextCompound::Raw(a) => a.clone(),
            TextCompound::Link(a, b) => (format!("[{}]({})", a.markdown()?, b)),
            TextCompound::Italic(a) => (format!("*{}*", a.markdown()?)),
            TextCompound::Bold(a) => (format!("**{}**", a.markdown()?)),
            TextCompound::Array(a) => a
                .iter()
                .flat_map(|x| x.markdown())
                .collect::<Vec<_>>()
                .join(""),
            TextCompound::Abbr(a, b) => (format!("{} (*{}*)", a.markdown()?, b)),
            TextCompound::Sup(a) => (format!("^{}^", a.markdown()?)),
            TextCompound::Sub(a) => (format!("~{}~", a.markdown()?)),
            TextCompound::Small(a) => (format!("~^{}^~", a.markdown()?)),
            TextCompound::Br => "\n".to_owned(),
            TextCompound::Code(a) => (format!("\n```\n{}\n```\n", a)),
            TextCompound::Img(a) => (format!("![{}]({})", a, a)),
            Self::H(_, a, b) => {
                format!(
                    "{} {}",
                    match a {
                        Header::H1 => "#",
                        Header::H2 => "##",
                        Header::H3 => "###",
                        Header::H4 => "####",
                        Header::H5 => "#####",
                    },
                    b.markdown()?
                )
            }

            Self::Ul(a) => a
                .iter()
                .flat_map(|x| Some(format!(" - {}", x.markdown()?)))
                .collect::<Vec<_>>()
                .join(""),
            Self::P(a) => a.markdown()?,
            Self::Table(a) => {
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
                    construct
                } else {
                    String::new()
                }
            }
            Self::Quote(a) => (format!(" > {}", a.markdown()?)),
        };
        if k.trim().is_empty() && !matches!(self, TextCompound::Br) {
            return None;
        }
        Some(k)
    }
    pub fn html(&self, ctx: &Context) -> Option<String> {
        let k: String = match self {
            TextCompound::Raw(a) => (format!("{} ", html_escape::encode_text(a))),
            TextCompound::Link(a, b) => {
                let a = a.html(ctx)?;
                if b.starts_with("#") {
                    format!("<a href=\"{}\">{}</a>", b, a)
                } else if is_text(b.as_ref()) && !a.contains("Official website") {
                    format!("<a href=\"/m/{}\">{}</a>", get_shortened_from_url(b), a)
                } else {
                    format!("<a href=\"{}\">{}</a>", b, a)
                }
            }
            TextCompound::Italic(a) => (format!("<i>{} </i>", a.html(ctx)?)),
            TextCompound::Bold(a) => (format!("<b>{} </b>", a.html(ctx)?)),
            TextCompound::Array(a) => a
                .iter()
                .flat_map(|x| x.html(ctx))
                .collect::<Vec<_>>()
                .join(""),
            TextCompound::Abbr(a, b) => (format!("<abbr title=\"{}\">{} </abbr>", b, a.html(ctx)?)),
            TextCompound::Sup(a) => (format!("<sup>{} </sup>", a.html(ctx)?)),
            TextCompound::Sub(a) => (format!("<sub>{} </sub>", a.html(ctx)?)),
            TextCompound::Small(a) => (format!("<small>{} </small>", a.html(ctx)?)),
            TextCompound::Br => ("<br/>".to_owned()),
            TextCompound::Code(a) => {
                if a.contains('\n') {
                    format!("<pre><code>{}</code></pre>", html_escape::encode_text(a))
                } else {
                    format!("<code>{}&nbsp;</code>", html_escape::encode_text(a))
                }
            }
            TextCompound::Img(a) => (format!("<img src=\"{}\">", a)),
            Self::H(c, a, b) => {
                let c: Vec<String> = c
                    .iter()
                    .flat_map(|x| ctx.map.get(x))
                    .map(|x| format!("#{}", x))
                    .collect();
                let header = match a {
                    Header::H1 => 1,
                    Header::H2 => 2,
                    Header::H3 => 3,
                    Header::H4 => 4,
                    Header::H5 => 5,
                };
                format!(
                    "<h{}{}>{}</h{}>",
                    header,
                    if c.is_empty() {
                        String::new()
                    } else {
                        format!(" id=\"{}\"", c.join(" "))
                    },
                    b.html(ctx)?,
                    header
                )
            }
            Self::Ul(a) => {
                format!(
                    "<ul>{}</ul>",
                    a.iter()
                        .flat_map(|x| Some(format!("<li>{}</li>", x.html(ctx)?)))
                        .collect::<Vec<_>>()
                        .join("")
                )
            }

            Self::P(a) => (format!("<p>{}</p>", a.html(ctx)?)),
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
                string
            }
            Self::Quote(a) => (format!("<quote>{}</quote>", a.html(ctx)?)),
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
