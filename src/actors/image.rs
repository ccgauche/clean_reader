use std::path::PathBuf;
use std::sync::mpsc::SyncSender;

use ractor::{Actor, ActorProcessingErr, ActorRef};

use crate::image::encode_avif;
use crate::utils::http_get_bytes;

pub enum ImageMsg {
    Encode {
        url: String,
        cache_path: PathBuf,
        done: SyncSender<()>,
    },
}

pub struct ImageActor;
pub struct ImageState;

impl Actor for ImageActor {
    type Msg = ImageMsg;
    type State = ImageState;
    type Arguments = ();

    async fn pre_start(
        &self,
        _myself: ActorRef<ImageMsg>,
        _args: (),
    ) -> std::result::Result<ImageState, ActorProcessingErr> {
        Ok(ImageState)
    }

    async fn handle(
        &self,
        _myself: ActorRef<ImageMsg>,
        msg: ImageMsg,
        _state: &mut ImageState,
    ) -> std::result::Result<(), ActorProcessingErr> {
        match msg {
            ImageMsg::Encode {
                url,
                cache_path,
                done,
            } => {
                // Fan out to an OS thread: both `http_get_bytes` (blocking
                // reqwest) and `encode_avif` (ravif + rayon) are CPU/IO-heavy
                // and benefit from a dedicated thread outside the tokio
                // executor.
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
            }
        }
        Ok(())
    }
}
