use std::{borrow::Cow, str::FromStr};

mod compiler;
mod display;
mod parser;

#[derive(Debug)]
pub enum Header {
    H1,
    H2,
    H3,
    H4,
    H5,
}

impl FromStr for Header {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "h1" => Self::H1,
            "h2" => Self::H2,
            "h3" => Self::H3,
            "h4" => Self::H4,
            "h5" => Self::H5,
            _ => return Err("Invalid header"),
        })
    }
}

#[derive(Debug)]
pub enum TextCompound<'a> {
    Raw(Cow<'a, str>),
    Link(Box<TextCompound<'a>>, Cow<'a, str>),
    Italic(Box<TextCompound<'a>>),
    Bold(Box<TextCompound<'a>>),
    Underline(Box<TextCompound<'a>>),
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

impl TextCompound<'_> {
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
}
