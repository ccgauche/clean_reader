pub mod nsi;

use std::collections::HashMap;

use crate::html_node::HTMLNode;

pub fn choose(node: &HTMLNode) -> &HTMLNode {
    let mut cache = HashMap::new();
    let global = Stats::from_node(node, &mut cache);
    let mut current_weight = (node, score_between(&global, &global));
    fn reccur<'a>(
        node: &'a HTMLNode,
        global: &Stats,
        best: &mut (&'a HTMLNode, f32),
        cache: &mut HashMap<u64, Stats>,
    ) {
        let current_weight = score_between(global, &Stats::from_node(node, cache));
        if current_weight.is_normal() {
            if current_weight > best.1 {
                *best = (node, current_weight);
            }
            match node {
                HTMLNode::Node(_, _, children) => {
                    for child in children {
                        reccur(child, global, best, cache);
                    }
                }
                HTMLNode::Text(_) => (),
            }
        }
    }
    reccur(node, &global, &mut current_weight, &mut cache);
    current_weight.0
}

#[derive(Clone)]
pub struct Stats {
    // Depth from current node, TextBlock itself
    text_blocks: usize,
    element_count: usize,
}

const WHITELIST: &[&str] = &[
    "div", "section", "article", "header", "footer", "nav", "aside", "main",
];

fn score_between(global_stats: &Stats, current_stats: &Stats) -> f32 {
    let global_element_count = global_stats.element_count;
    let local_element_count = current_stats.element_count;
    let ratio_of_page = local_element_count as f32 / global_element_count as f32;
    let ratio_of_text_blocks = current_stats.text_blocks as f32 / global_stats.text_blocks as f32;
    ratio_of_text_blocks / ratio_of_page
}

const TEXT_BLOCK_THREASHOLD: usize = 200;

impl Stats {
    pub fn from_node(node: &HTMLNode, cache: &mut HashMap<u64, Stats>) -> Self {
        let k = node.hashcode();
        if let Some(e) = cache.get(&k) {
            return e.clone();
        }
        let out = match node {
            HTMLNode::Node(a, _, c) => {
                let mut text_blocks = 0;
                if a == "h1"
                    || ((a == "h2" || a == "h3")
                        && c.iter().map(|x| x.get_text().len()).sum::<usize>() > 10)
                {
                    text_blocks += 1;
                }
                let mut element_count = 0;
                if WHITELIST.contains(&a.as_str()) && !c.iter().any(|n| !n.is_text()) {
                    element_count += 1;
                }
                for child in c {
                    let child_stats = Stats::from_node(child, cache);
                    text_blocks += child_stats.text_blocks;
                    element_count += child_stats.element_count;
                }
                Self {
                    text_blocks,
                    element_count,
                }
            }
            HTMLNode::Text(a) => {
                if a.trim().len() > TEXT_BLOCK_THREASHOLD {
                    Self {
                        text_blocks: 1,
                        element_count: 0,
                    }
                } else {
                    Self {
                        text_blocks: 0,
                        element_count: 0,
                    }
                }
            }
        };
        cache.insert(k, out.clone());
        out
    }
}
