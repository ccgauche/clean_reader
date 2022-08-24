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

impl Header {
    fn to_str(&self) -> &'static str {
        match self {
            Self::H1 => "h1",
            Self::H2 => "h2",
            Self::H3 => "h3",
            Self::H4 => "h4",
            Self::H5 => "h5",
        }
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
    #[allow(dead_code)]
    pub fn name(&self) -> &'static str {
        match self {
            TextCompound::Raw(_) => "raw",
            TextCompound::Link(_, _) => "link",
            TextCompound::Italic(_) => "italic",
            TextCompound::Bold(_) => "bold",
            TextCompound::Underline(_) => "underline",
            TextCompound::Array(_) => "array",
            TextCompound::Abbr(_, _) => "abbr",
            TextCompound::Sup(_) => "sup",
            TextCompound::Sub(_) => "sub",
            TextCompound::Small(_) => "small",
            TextCompound::Code(_) => "code",
            TextCompound::Img(_) => "img",
            TextCompound::Br => "br",
            TextCompound::H(_, _, _) => "h",
            TextCompound::P(_) => "p",
            TextCompound::Quote(_) => "quote",
            TextCompound::Ul(_) => "ul",
            TextCompound::Table(_) => "table",
        }
    }
    pub fn contains_title(&self) -> bool {
        match self {
            Self::Array(e) => {
                matches!(e.get(0), Some(Self::H(_, Header::H1, _)))
            }
            _ => false,
        }
    }
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
