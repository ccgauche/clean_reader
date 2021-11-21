use std::borrow::{Borrow, Cow};

use crate::text_element::{Header, TextCompound};

impl<'a> TextCompound<'a> {
    #[allow(unused)]
    pub fn markdown(&'a self) -> Option<Cow<'a, str>> {
        let k = match self {
            TextCompound::Raw(a) => Cow::Borrowed(a.borrow()),
            TextCompound::Link(a, b) => Cow::Owned(format!("[{}]({})", a.markdown()?, b)),
            TextCompound::Italic(a) => Cow::Owned(format!("*{}*", a.markdown()?)),
            TextCompound::Bold(a) => Cow::Owned(format!("**{}**", a.markdown()?)),
            TextCompound::Underline(a) => Cow::Owned(format!("**{}**", a.markdown()?)),
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
}
