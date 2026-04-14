/// Errors from the HTML-tree lowering stage ([`crate::html_node::HTMLNode::from_handle`]).
///
/// These are used as pruning signals as much as hard failures — the
/// parent walker collects children via `flat_map` and silently drops
/// nodes that error out, on the theory that a bad `<script>` tag
/// shouldn't kill the whole article.
#[derive(Debug, thiserror::Error)]
pub enum NodeError {
    #[error("Skipping <{tag}>: tag is in the blocklist")]
    BlockedTag { tag: String },

    #[error("Skipping <{tag}>: no children")]
    EmptyNode { tag: String },

    #[error("Skipping empty text node")]
    EmptyText,

    #[error("Skipping comment node")]
    CommentNode,
}
