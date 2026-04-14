use super::TextCompound;

/// One cell inside a `<table>`. `Header` maps to `<th>`, `Data` to `<td>` —
/// there are no other kinds, which is exactly what `enum` is for. The
/// previous representation was `(bool, TextCompound)`, which required the
/// reader to remember which way the boolean pointed.
#[derive(Debug)]
pub enum TableCell<'a> {
    Header(TextCompound<'a>),
    Data(TextCompound<'a>),
}

impl<'a> TableCell<'a> {
    pub fn content(&self) -> &TextCompound<'a> {
        match self {
            Self::Header(content) | Self::Data(content) => content,
        }
    }

    pub fn html_tag(&self) -> &'static str {
        match self {
            Self::Header(_) => "th",
            Self::Data(_) => "td",
        }
    }
}
