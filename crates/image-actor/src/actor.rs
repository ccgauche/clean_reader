use std::path::Path;

use ractor::{Actor, ActorProcessingErr, ActorRef};
use reader_core::image::{encode_avif, ImageError};
use reader_core::utils::http_get_bytes;

use crate::message::ImageMsg;

/// ractor actor that owns the image re-encode path. On each message it
/// spawns a dedicated OS thread: both `http_get_bytes` (blocking reqwest)
/// and `encode_avif` (ravif + rayon) are CPU/IO heavy and benefit from
/// running outside the tokio executor.
pub struct ImageActor;

impl Actor for ImageActor {
    type Msg = ImageMsg;
    type State = ();
    type Arguments = ();

    async fn pre_start(
        &self,
        _myself: ActorRef<ImageMsg>,
        _args: (),
    ) -> std::result::Result<(), ActorProcessingErr> {
        Ok(())
    }

    async fn handle(
        &self,
        _myself: ActorRef<ImageMsg>,
        msg: ImageMsg,
        _state: &mut (),
    ) -> std::result::Result<(), ActorProcessingErr> {
        let ImageMsg::Encode {
            url,
            cache_path,
            done,
        } = msg;
        std::thread::spawn(move || {
            if let Err(e) = fetch_encode_write(&url, &cache_path) {
                eprintln!("image worker {}: {}", url, e);
            }
            let _ = done.send(());
        });
        Ok(())
    }
}

/// Pure linear pipeline: fetch, encode, write. Using `?` keeps nesting
/// flat — the old version had `match` inside `match` inside `if let` at
/// four levels deep.
fn fetch_encode_write(url: &str, cache_path: &Path) -> Result<(), ImageError> {
    let bytes = http_get_bytes(url)?;
    let avif = encode_avif(&bytes)?;
    if let Some(parent) = cache_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(cache_path, &avif)?;
    Ok(())
}
