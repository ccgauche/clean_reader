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
                string.push_str("<a href=\"");
                if !ctx.download
                    && !b.starts_with("mailto:")
                    && !b.starts_with('#')
                    && is_html(b.as_ref())
                {
                    string.push_str("/m/");
                    string.push_str(&get_shortened_from_url(b));
                } else {
                    string.push_str(b);
                }
                string.push_str("\">");
                a.html(ctx, string);
                string.push_str("</a>");
            }
            TextCompound::Italic(a) => {
                string.push_str("<i>");
                a.html(ctx, string);
                string.push_str("</i>");
            }
            TextCompound::Bold(a) => {
                string.push_str("<b>");
                a.html(ctx, string);
                string.push_str("</b>");
            }
            TextCompound::Array(a) => a.iter().for_each(|x| x.html(ctx, string)),
            TextCompound::Abbr(a, b) => {
                string.push_str("<abbr title=\"");
                string.push_str(b);
                string.push_str("\">");
                a.html(ctx, string);
                string.push_str("</abbr>");
            }
            TextCompound::Sup(a) => {
                string.push_str("<sup>");
                a.html(ctx, string);
                string.push_str("</sup>");
            }
            TextCompound::Sub(a) => {
                string.push_str("<sub>");
                a.html(ctx, string);
                string.push_str("</sub>");
            }
            TextCompound::Small(a) => {
                string.push_str("<small>");
                a.html(ctx, string);
                string.push_str("</small>");
            }
            TextCompound::Br => {
                string.push_str("<br/>");
            }
            TextCompound::Code(a) => {
                if a.contains('\n') {
                    string.push_str("<pre><code>");
                    string.push_str(&html_escape::encode_text(a));
                    string.push_str("</code></pre>");
                } else {
                    string.push_str("<code>");
                    string.push_str(&html_escape::encode_text(a));
                    string.push_str("&nbsp;</code>");
                }
            }

            TextCompound::Img(a) => {
                string.push_str("<img src=\"");
                string.push_str(&a);
                string.push_str("\">");
            }
            Self::H(c, a, b) => {
                let c: Vec<String> = c
                    .iter()
                    .flat_map(|x| ctx.map.get(x.as_ref()))
                    .map(|x| format!("#{}", x))
                    .collect();
                let header = match a {
                    Header::H1 => '1',
                    Header::H2 => '2',
                    Header::H3 => '3',
                    Header::H4 => '4',
                    Header::H5 => '5',
                };

                string.push_str("<h");
                string.push(header);
                if !c.is_empty() {
                    string.push_str(" id=\"");
                    string.push_str(&c.join(" "));
                    string.push('"');
                }
                string.push('>');
                b.html(ctx, string);
                string.push_str("</h");
                string.push(header);
                string.push('>');
            }
            Self::Ul(a) => {
                string.push_str("<ul>");
                for x in a {
                    string.push_str("<li>");
                    x.html(ctx, string);
                    string.push_str("</li>");
                }
                string.push_str("</ul>");
            }

            Self::P(a) => {
                string.push_str("<p>");
                a.html(ctx, string);
                string.push_str("</p>");
            }
            Self::Table(a) => {
                string.push_str("<table>");
                for i in a {
                    string.push_str("<tr>");
                    for (a, b) in i {
                        string.push_str(if *a { "<th>" } else { "<td>" });
                        b.html(ctx, string);
                        string.push_str(if *a { "</th>" } else { "</td>" });
                    }
                    string.push_str("</tr>");
                }
                string.push_str("</table>");
            }
            Self::Quote(a) => {
                string.push_str("<quote>");
                a.html(ctx, string);
                string.push_str("</quote>");
            }
        }
    }
}
