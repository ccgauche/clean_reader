/// How a rendered article is intended to be consumed.
///
/// `View` means the article is served back by the same Clean Reader
/// server, so outbound links get rewritten through `/m/` for one-click
/// cleaning. `Download` means the HTML is a self-contained file the user
/// is taking off the server, so links keep their original targets and the
/// "download this article" footer is suppressed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RenderMode {
    View,
    Download,
}

impl RenderMode {
    pub fn is_download(self) -> bool {
        matches!(self, Self::Download)
    }
}
