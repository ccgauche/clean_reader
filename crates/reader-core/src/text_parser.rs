use std::{borrow::Cow, collections::HashMap};

use reqwest::Url;

use crate::title_extractor::ArticleData;

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

/// Mutable context threaded through the text-compound lowering and
/// HTML-compilation passes. Holds the source URL (for link absolutization),
/// the render mode, an anchor-renaming map, and the page metadata.
#[derive(Clone)]
pub struct Context<'a> {
    pub url: Url,
    pub mode: RenderMode,
    pub min_id: String,
    pub map: HashMap<&'a str, usize>,
    pub count: usize,
    pub meta: ArticleData,
}

impl<'a> Context<'a> {
    /// Resolve a potentially-relative link against the article URL and
    /// rewrite `#fragment` anchors to dedup-friendly numeric ids.
    pub fn absolutize(&mut self, url: &'a str) -> Cow<'a, str> {
        if let Some(fragment) = url.strip_prefix('#') {
            if let Some(existing) = self.map.get(fragment) {
                Cow::Owned(format!("#{}", existing))
            } else {
                self.count += 1;
                self.map.insert(fragment, self.count);
                Cow::Owned(format!("#{}", self.count))
            }
        } else {
            self.url
                .join(url)
                .map(|joined| Cow::Owned(joined.to_string()))
                .unwrap_or_else(|_| Cow::Borrowed(url))
        }
    }
}
