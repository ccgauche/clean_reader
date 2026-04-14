use ractor::RpcReplyPort;
use reader_core::pipeline_error::PipelineError;
use reader_core::render_mode::RenderMode;

/// Request messages accepted by the [`super::actor::PageActor`].
pub enum PageMsg {
    /// Fetch a URL and render it through the reader pipeline. The reply
    /// port carries the rendered HTML (or the pipeline error).
    Render {
        url: String,
        min_id: String,
        mode: RenderMode,
        reply: RpcReplyPort<Result<String, PipelineError>>,
    },
}
