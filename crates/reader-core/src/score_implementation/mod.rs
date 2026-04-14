use crate::html_node::HTMLNode;

/// Three-way result used by the leading-image scan: either we hit an
/// image first, we hit real text first, or we ran off the end of the
/// subtree without seeing either.
enum ScanResult {
    Image,
    Text,
    Empty,
}

/// Whether the first non-empty descendant the walker reaches is an
/// `<img>`. Used to decide whether the hero image from page metadata is
/// redundant with the one Readability already preserved inside the
/// article body.
pub fn starts_with_image(node: &HTMLNode) -> bool {
    matches!(walk(node), ScanResult::Image)
}

fn walk(node: &HTMLNode) -> ScanResult {
    match node {
        HTMLNode::Element { tag, children, .. } => walk_element(tag, children),
        HTMLNode::Text(text) => walk_text(text),
    }
}

fn walk_element(tag: &str, children: &[HTMLNode]) -> ScanResult {
    if tag == "img" {
        return ScanResult::Image;
    }
    children
        .iter()
        .map(walk)
        .find(|result| !matches!(result, ScanResult::Empty))
        .unwrap_or(ScanResult::Empty)
}

fn walk_text(text: &str) -> ScanResult {
    if text.trim().is_empty() {
        ScanResult::Empty
    } else {
        ScanResult::Text
    }
}
