use crate::html_node::HTMLNode;

/**

This function computes if the HTMLNode contains an image before any text

return:0 = Contains image
return:1 = Has reached text

*/
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

/**

This function tries to extract the title from the article

return:0 = Contains the found node
return:1 = The title text found
*/
pub fn extract_title(node: &HTMLNode, choosen: &HTMLNode) -> (bool, Option<String>) {
    if node == choosen {
        (true, None)
    } else if let HTMLNode::Node(tag, _, children) = &node {
        (
            false,
            if tag == "h1" {
                Some(node.get_text())
            } else {
                let mut e = None;
                for child in children {
                    let (found, title) = extract_title(child, choosen);
                    e = title.and(e);
                    if found {
                        return (true, e);
                    }
                }
                e
            },
        )
    } else {
        (false, None)
    }
}

/**
 * Choose the best node to use for content extraction
 */
pub fn choose(node: &HTMLNode) -> &HTMLNode {
    fn inner(node: &HTMLNode) -> (&HTMLNode, usize) {
        let current_score = node_score(node);
        if let HTMLNode::Node(_, _, x) = node {
            x.iter().fold((node, current_score), |best, child| {
                let score = inner(child);
                if score.1 > best.1 {
                    score
                } else {
                    best
                }
            })
        } else {
            (node, current_score)
        }
    }
    inner(node).0
}

const IS_DIV_LIKE: &[&str] = &[
    "div", "section", "article", "header", "footer", "nav", "aside", "main", "li",
];

const IGNORE_ELEMENTS: &[&str] = &["a", "li"];

fn should_be_ignored(string: Option<&str>) -> bool {
    string
        .map(|x| IGNORE_ELEMENTS.contains(&x))
        .unwrap_or(false)
}
fn is_container(string: Option<&str>) -> bool {
    string.map(|x| IS_DIV_LIKE.contains(&x)).unwrap_or(false)
}
/**
 * This function is used by the choose function in intern to generate a score to compare nodes.
 */
fn node_score(node: &HTMLNode) -> usize {
    match node {
        HTMLNode::Node(tag, _, c) => c
            .iter()
            .map(if is_container(Some(tag.as_str())) {
                |x: &HTMLNode| {
                    if should_be_ignored(x.get_tag_name()) {
                        0
                    } else if is_container(x.get_tag_name()) {
                        node_score(x) / 2
                    } else {
                        node_score(x)
                    }
                }
            } else {
                node_score
            })
            .sum(),
        HTMLNode::Text(text) => (text.len() / 100).max(30),
    }
}
