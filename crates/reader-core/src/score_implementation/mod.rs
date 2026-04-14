use crate::html_node::HTMLNode;

/// Returns `(has_leading_image, reached_text)`.
///
/// `has_leading_image` is true when the first non-empty descendant the
/// walker reaches is an `<img>`. Used to decide whether the hero image
/// from page metadata is redundant with the one Readability already
/// preserved inside the article body.
pub fn contains_image(node: &HTMLNode) -> (bool, bool) {
    match node {
        HTMLNode::Element { tag, children, .. } => {
            if tag == "img" {
                return (true, false);
            }
            for child in children {
                let (has_image, reached_text) = contains_image(child);
                if has_image {
                    return (true, true);
                }
                if reached_text {
                    return (false, true);
                }
            }
            (false, false)
        }
        HTMLNode::Text(text) => (false, text.trim().is_empty()),
    }
}
