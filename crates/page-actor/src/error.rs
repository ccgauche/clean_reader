use reader_core::pipeline_error::PipelineError;

/// Errors returned by [`super::render_page`] and [`super::boot`].
///
/// `Pipeline` wraps the narrow error returned by
/// [`reader_core::pipeline::render`]; the other variants cover actor-system
/// failures (not booted, call dropped, timeout) that are specific to the
/// ractor wrapping.
#[derive(Debug, thiserror::Error)]
pub enum PageActorError {
    #[error("page actor not booted")]
    NotBooted,

    #[error("page actor spawn failed: {0}")]
    SpawnFailed(String),

    #[error("page actor call failed: {0}")]
    CallFailed(String),

    #[error("page actor reply dropped")]
    ReplyDropped,

    #[error("page actor timeout")]
    Timeout,

    #[error(transparent)]
    Pipeline(#[from] PipelineError),
}
