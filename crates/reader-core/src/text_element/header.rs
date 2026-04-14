use std::str::FromStr;

/// HTML heading level. The reader collapses anything past `h5` because
/// `h6` rarely carries meaningful structure in practice.
#[derive(Debug)]
pub enum Header {
    H1,
    H2,
    H3,
    H4,
    H5,
}

impl Header {
    pub fn to_str(&self) -> &'static str {
        match self {
            Self::H1 => "h1",
            Self::H2 => "h2",
            Self::H3 => "h3",
            Self::H4 => "h4",
            Self::H5 => "h5",
        }
    }
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
