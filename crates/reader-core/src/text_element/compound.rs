use std::borrow::Cow;

use super::{Header, Table};

/// Rich-text IR produced by the parser stage and consumed by the HTML
/// template compiler.
///
/// `TextCompound` is intentionally small — adding a new variant means
/// updating the parser (`from_node`), the compiler (`html`), the plain-text
/// projection (`text`), and likely the article template. Keep the surface
/// narrow.
///
/// Variants with multiple fields use struct syntax so every call site
/// reads `Link { content, href }` rather than `Link(_, _)` and having to
/// remember which position is which.
#[derive(Debug)]
pub enum TextCompound<'a> {
    Raw(Cow<'a, str>),
    Link {
        content: Box<TextCompound<'a>>,
        href: Cow<'a, str>,
    },
    Italic(Box<TextCompound<'a>>),
    Bold(Box<TextCompound<'a>>),
    Underline(Box<TextCompound<'a>>),
    Array(Vec<TextCompound<'a>>),
    Abbr {
        content: Box<TextCompound<'a>>,
        title: Cow<'a, str>,
    },
    Sup(Box<TextCompound<'a>>),
    Sub(Box<TextCompound<'a>>),
    Small(Box<TextCompound<'a>>),
    Code(String),
    Img(Cow<'a, str>),
    Br,
    Heading {
        fragment_ids: Vec<Cow<'a, str>>,
        level: Header,
        content: Box<TextCompound<'a>>,
    },
    P(Box<TextCompound<'a>>),
    Quote(Box<TextCompound<'a>>),
    Ul(Vec<TextCompound<'a>>),
    Table(Table<'a>),
}

impl TextCompound<'_> {
    /// Whether the first element of an `Array` is an `H1`. Used by the
    /// pipeline to decide whether to dedup the page title against the
    /// article's own leading heading.
    pub fn contains_title(&self) -> bool {
        match self {
            Self::Array(items) => matches!(
                items.first(),
                Some(Self::Heading {
                    level: Header::H1,
                    ..
                })
            ),
            _ => false,
        }
    }

    /// Drop a leading `H1` from an `Array` so the page title and the
    /// article's own title don't render twice.
    pub fn remove_title(self) -> Self {
        match self {
            Self::Array(mut items) => {
                if matches!(
                    items.first(),
                    Some(Self::Heading {
                        level: Header::H1,
                        ..
                    })
                ) {
                    items.remove(0);
                }
                Self::Array(items)
            }
            other => other,
        }
    }
}
