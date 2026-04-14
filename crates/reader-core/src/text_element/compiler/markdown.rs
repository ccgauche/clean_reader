use std::borrow::{Borrow, Cow};

use crate::text_element::{Header, TextCompound};

impl<'a> TextCompound<'a> {
    #[allow(unused)]
    pub fn markdown(&'a self) -> Option<Cow<'a, str>> {
        let k = match self {
            Self::Raw(a) => Cow::Borrowed(a.borrow()),
            Self::Link(a, b) => Cow::Owned(format!("[{}]({})", a.markdown()?, b)),
            Self::Italic(a) => Cow::Owned(format!("*{}*", a.markdown()?)),
            Self::Bold(a) => Cow::Owned(format!("**{}**", a.markdown()?)),
            Self::Underline(a) => Cow::Owned(format!("**{}**", a.markdown()?)),
            Self::Array(a) => Cow::Owned(
                a.iter()
                    .flat_map(|x| x.markdown())
                    .collect::<Vec<_>>()
                    .join(""),
            ),
            Self::Abbr(a, b) => Cow::Owned(format!("{} (*{}*)", a.markdown()?, b)),
            Self::Sup(a) => Cow::Owned(format!("^{}^", a.markdown()?)),
            Self::Sub(a) => Cow::Owned(format!("~{}~", a.markdown()?)),
            Self::Small(a) => Cow::Owned(format!("~^{}^~", a.markdown()?)),
            Self::Br => Cow::Borrowed("\n"),
            Self::Code(a) => Cow::Owned(format!("\n```\n{}\n```\n", a)),
            Self::Img(a) => Cow::Owned(format!("![{}]({})", a, a)),
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
        if k.trim().is_empty() && !matches!(self, Self::Br) {
            None
        } else {
            Some(k)
        }
    }
}
