use crate::html_node::HTMLNode;

pub fn choose(node: &HTMLNode) -> &HTMLNode {
    _choose(node).0
}

/*
r:0 = Is image
r:1 = Reached text */
pub fn contains_image(node: &HTMLNode) -> (bool, bool) {
    match node {
        HTMLNode::Node(a, _, c) => {
            if a == "img" {
                return (true, false);
            }

            for child in c {
                let (r, p) = contains_image(child);
                if r {
                    return (true, true);
                } else if p {
                    return (false, true);
                }
            }
            (false, false)
        }
        HTMLNode::Text(t) => (false, t.trim().is_empty()),
    }
}

/*
r:0 = Contains the found node
r:1 = The title node found
*/
pub fn extract_title(node: &HTMLNode, choosen: &HTMLNode) -> (bool, Option<String>) {
    if node == choosen {
        return (true, None);
    }
    match node {
        HTMLNode::Node(tag, _, children) => {
            if tag == "h1" {
                return (false, Some(node.get_text()));
            }
            let mut e = None;
            for child in children.iter() {
                let (found, title) = extract_title(child, choosen);
                e = title.and(e);
                if found {
                    return (true, e);
                }
            }
            (false, e)
        }
        HTMLNode::Text(_) => (false, None),
    }
}

fn _choose(node: &HTMLNode) -> (&HTMLNode, usize) {
    let current_score = node_score(node);
    match node {
        HTMLNode::Node(_, _, x) => {
            let mut best = (node, current_score);
            for child in x {
                let score = _choose(child);
                if score.1 > best.1 {
                    best = (score.0, score.1);
                }
            }
            best
        }
        HTMLNode::Text(_) => (node, current_score),
    }
}

const IS_DIV_LIKE: &[&str] = &[
    "div", "section", "article", "header", "footer", "nav", "aside", "main", "li",
];

const IGNORE_ELEMENTS: &[&str] = &["a", "li"];

fn node_score(node: &HTMLNode) -> usize {
    match node {
        HTMLNode::Node(ref tag, _, c) => {
            if IS_DIV_LIKE.contains(&tag.as_str()) {
                c.iter()
                    .map(|x| {
                        if x.get_node_element()
                            .map(|x| IGNORE_ELEMENTS.contains(&x))
                            .unwrap_or(false)
                        {
                            0
                        } else if x
                            .get_node_element()
                            .map(|x| IS_DIV_LIKE.contains(&x))
                            .unwrap_or(false)
                        {
                            node_score(x) / 2
                        } else {
                            node_score(x)
                        }
                    })
                    .sum()
            } else {
                c.iter().map(node_score).sum()
            }
        }
        HTMLNode::Text(text) => (text.len() / 100).max(30),
    }
}
