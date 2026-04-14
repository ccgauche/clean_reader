use std::path::PathBuf;
use std::sync::mpsc::SyncSender;

/// Request messages accepted by the [`super::actor::ImageActor`].
pub enum ImageMsg {
    /// Fetch `url`, re-encode it to AVIF, and write the result to
    /// `cache_path`. `done` is signalled when the worker has finished
    /// (success or failure) so the caller can bound its wait.
    Encode {
        url: String,
        cache_path: PathBuf,
        done: SyncSender<()>,
    },
}
