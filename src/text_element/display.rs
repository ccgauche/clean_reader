use std::fmt::Display;

use crate::text_element::{Header, TextCompound};

impl Display for TextCompound<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Raw(a) => write!(f, "Raw({})", a),
            Self::Link(a, b) => write!(f, "Raw({},{})", a, b),
            Self::Italic(a) => write!(f, "Italic({})", a),
            Self::Bold(a) => write!(f, "Bold({})", a),
            Self::Underline(a) => write!(f, "Underline({})", a),
            Self::Array(a) => write!(
                f,
                "{}",
                a.iter()
                    .map(|x| x.to_string())
                    .collect::<Vec<_>>()
                    .join("+")
            ),
            Self::Abbr(a, b) => write!(f, "Abbr({},{})", a, b),
            Self::Sup(a) => write!(f, "Sup({})", a),
            Self::Sub(a) => write!(f, "Sub({})", a),
            Self::Small(a) => write!(f, "Small({})", a),
            Self::Code(a) => write!(f, "Code({})", a),
            Self::Img(a) => write!(f, "Img({})", a),
            Self::Br => write!(f, "Br()"),
            Self::H(a, b, c) => write!(
                f,
                "H{}([{}],{})",
                match b {
                    Header::H1 => "1",
                    Header::H2 => "2",
                    Header::H3 => "3",
                    Header::H4 => "4",
                    Header::H5 => "5",
                },
                a.iter()
                    .map(|x| x.to_string())
                    .collect::<Vec<_>>()
                    .join(","),
                c
            ),
            Self::P(a) => write!(f, "P({})", a),
            Self::Quote(a) => write!(f, "Quote({})", a),
            Self::Ul(a) => write!(
                f,
                "Ul({})",
                a.iter()
                    .map(|x| x.to_string())
                    .collect::<Vec<_>>()
                    .join(",")
            ),
            Self::Table(a) => write!(
                f,
                "Table([{}])",
                a.iter()
                    .map(|x| x
                        .iter()
                        .map(|x| x.1.to_string())
                        .collect::<Vec<_>>()
                        .join(","))
                    .collect::<Vec<_>>()
                    .join("],[")
            ),
        }
    }
}
