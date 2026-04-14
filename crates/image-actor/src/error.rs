/// Errors returned by [`super::boot`]. Image re-encode failures inside
/// the actor's handle path are logged and swallowed — they never
/// propagate out.
#[derive(Debug, thiserror::Error)]
pub enum ImageActorError {
    #[error("image actor spawn failed: {0}")]
    SpawnFailed(String),
}
