use crate::html_node::HTMLNode;

const ELEMENTS: &[(&[&str], f32)] = &[
    (
        &["h1", "h2", "h3", "h4", "h5", "h6", "blockquote", "cite"],
        4.,
    ),
    (&["p", "br"], 10.),
    (&["ul"], -2.),
];

const TEXT_SCORE: f32 = 30.;
const MAX_THRESHOLD: f32 = 5000.;

/// This recursive function finds the best node in the tree (The one which is the likeliest to contains interesting content)
// TODO(performance high): Propagate score evalutation
pub fn best_node(nodes: &HTMLNode) -> &HTMLNode {
    let mut score = nodes.score();
    let mut node = nodes;
    match nodes {
        HTMLNode::Node(_, _, c) => {
            for i in c {
                let i = best_node(i);
                let s = i.score();
                if s > score {
                    score = s;
                    node = i;
                }
            }
        }
        HTMLNode::Text(_) => (),
    }
    node
}

impl HTMLNode {
    /*
    TODO(performance high): Propagate score evalutation or add some cache
     */
    fn score(&self) -> f32 {
        match self {
            HTMLNode::Node(a, _, c) => {
                c.iter().map(|x| x.score() * 0.7).sum::<f32>()
                    + ELEMENTS
                        .iter()
                        .find(|(c, _)| c.contains(&a.as_str()))
                        .map(|x| x.1)
                        .unwrap_or(0.)
            }
            HTMLNode::Text(a) => (a.len() as f32).min(MAX_THRESHOLD) / MAX_THRESHOLD * TEXT_SCORE,
        }
    }
}
