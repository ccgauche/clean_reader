use ractor::{Actor, ActorProcessingErr, ActorRef};
use reader_core::image::encode_avif;
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
            match http_get_bytes(&url) {
                Ok(bytes) => match encode_avif(&bytes) {
                    Ok(avif) => {
                        if let Some(parent) = cache_path.parent() {
                            if let Err(e) = std::fs::create_dir_all(parent) {
                                eprintln!("mkdir {} failed: {}", parent.display(), e);
                                let _ = done.send(());
                                return;
                            }
                        }
                        if let Err(e) = std::fs::write(&cache_path, &avif) {
                            eprintln!("write {} failed: {}", cache_path.display(), e);
                        }
                    }
                    Err(e) => eprintln!("encode {}: {}", url, e),
                },
                Err(e) => eprintln!("fetch {}: {}", url, e),
            }
            let _ = done.send(());
        });
        Ok(())
    }
}
