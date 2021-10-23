use std::borrow::Cow;

use crate::{
    cache::get_shortened_from_url,
    text_element::{Header, TextCompound},
    text_parser::Context,
    utils::is_html,
};

impl<'a> TextCompound<'a> {
    pub fn html(&'a self, ctx: &Context) -> Option<Cow<'a, str>> {
        let k: Cow<'a, str> = match self {
            TextCompound::Raw(a) => html_escape::encode_text(a),
            TextCompound::Link(a, b) => {
                let a = a.html(ctx)?;
                Cow::Owned(
                    if !ctx.download
                        && !b.starts_with('#')
                        && is_html(b.as_ref())
                        && !a.contains("Official website")
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
