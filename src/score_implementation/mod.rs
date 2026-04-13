use crate::html_node::HTMLNode;

/// Returns `(has_leading_image, reached_text)`.
///
/// `has_leading_image` is true when the first non-empty descendant the walker
/// reaches is an `<img>`. Used to decide whether the hero image from page
/// metadata is redundant with the one Readability already preserved inside
/// the article body.
pub fn contains_image(node: &HTMLNode) -> (bool, bool) {
    match node {
        HTMLNode::Node(a, _, c) => (
            if a == "img" {
                true
            } else {
                for child in c {
                    let (r, p) = contains_image(child);
                    if r {
                        return (true, true);
                    } else if p {
                        return (false, true);
                    }
                }
                false
            },
            false,
        ),
        HTMLNode::Text(t) => (false, t.trim().is_empty()),
    }
}
