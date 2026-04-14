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

impl<'a> TextCompound<'a> {
    /// Construct a `Raw` text node from anything that can become a
    /// `Cow<'a, str>` — `&'a str`, `String`, or an existing `Cow`.
    pub fn raw(text: impl Into<Cow<'a, str>>) -> Self {
        Self::Raw(text.into())
    }

    /// Construct an `Img` from a URL-like value.
    pub fn img(src: impl Into<Cow<'a, str>>) -> Self {
        Self::Img(src.into())
    }

    /// Wrap `content` in an anchor with the given href.
    pub fn link(content: Self, href: impl Into<Cow<'a, str>>) -> Self {
        Self::Link {
            content: Box::new(content),
            href: href.into(),
        }
    }

    /// Wrap `content` in an `<abbr>` with the given `title` attribute.
    /// Pass an empty string when the source had no title.
    pub fn abbr(content: Self, title: impl Into<Cow<'a, str>>) -> Self {
        Self::Abbr {
            content: Box::new(content),
            title: title.into(),
        }
    }

    /// Construct a heading at `level` wrapping `content`. Accepts any
    /// iterable of string-like items for the fragment ids — `[]`, a
    /// `Vec<&str>`, a `Vec<String>`, or anything that yields
    /// `impl Into<Cow<'a, str>>`.
    pub fn heading<I, S>(level: Header, fragment_ids: I, content: Self) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<Cow<'a, str>>,
    {
        Self::Heading {
            fragment_ids: fragment_ids.into_iter().map(Into::into).collect(),
            level,
            content: Box::new(content),
        }
    }

    pub fn italic(content: Self) -> Self {
        Self::Italic(Box::new(content))
    }

    pub fn bold(content: Self) -> Self {
        Self::Bold(Box::new(content))
    }

    pub fn underline(content: Self) -> Self {
        Self::Underline(Box::new(content))
    }

    pub fn small(content: Self) -> Self {
        Self::Small(Box::new(content))
    }

    pub fn sub(content: Self) -> Self {
        Self::Sub(Box::new(content))
    }

    pub fn sup(content: Self) -> Self {
        Self::Sup(Box::new(content))
    }

    pub fn paragraph(content: Self) -> Self {
        Self::P(Box::new(content))
    }

    pub fn quote(content: Self) -> Self {
        Self::Quote(Box::new(content))
    }

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
