use std::borrow::Cow;

use super::Header;

/// Rich-text IR produced by the parser stage and consumed by the HTML
/// template compiler.
///
/// `TextCompound` is intentionally small — adding a new variant means
/// updating the parser (`from_node`), the compiler (`html`), the plain-text
/// projection (`text`), and likely the article template. Keep the surface
/// narrow.
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
    /// Whether the first element of an `Array` is an `H1`. Used by the
    /// pipeline to decide whether to dedup the page title against the
    /// article's own leading heading.
    pub fn contains_title(&self) -> bool {
        match self {
            Self::Array(items) => matches!(items.first(), Some(Self::H(_, Header::H1, _))),
            _ => false,
        }
    }

    /// Drop a leading `H1` from an `Array` so the page title and the
    /// article's own title don't render twice.
    pub fn remove_title(self) -> Self {
        match self {
            Self::Array(mut items) => {
                if matches!(items.first(), Some(Self::H(_, Header::H1, _))) {
                    items.remove(0);
                }
                Self::Array(items)
            }
            other => other,
        }
    }
}
