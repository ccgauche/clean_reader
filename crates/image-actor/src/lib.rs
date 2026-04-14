//! Image re-encoding actor.
//!
//! Wraps the CPU-heavy fetch + ravif encode in a ractor actor. At boot, it
//! registers a closure with [`reader_core::image::register_encoder`] so the
//! template renderer can trigger encoding without knowing about the actor
//! at all.

use std::path::PathBuf;
use std::sync::mpsc::{self, SyncSender};

use once_cell::sync::OnceCell;
use ractor::concurrency::JoinHandle;
use ractor::{Actor, ActorProcessingErr, ActorRef};
use reader_core::error::{Error, Result};
use reader_core::image::{encode_avif, register_encoder, EncoderFn, ImageTicket};
use reader_core::utils::http_get_bytes;

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
                // Fan out to an OS thread: both http_get_bytes (blocking
                // reqwest) and encode_avif (ravif + rayon) are CPU/IO heavy
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

/// Keep the actor JoinHandle alive for the lifetime of the process so the
/// actor isn't collected out from under us after `boot()` returns.
static ACTOR_HANDLE: OnceCell<JoinHandle<()>> = OnceCell::new();

/// Spawn the image actor and register its encoder with reader-core.
pub async fn boot() -> Result<()> {
    let (actor_ref, handle): (ActorRef<ImageMsg>, JoinHandle<()>) =
        Actor::spawn(Some("image".into()), ImageActor, ())
            .await
            .map_err(|e| Error::Actor(format!("spawn ImageActor: {}", e)))?;
    let _ = ACTOR_HANDLE.set(handle);

    let encoder: EncoderFn = Box::new(move |url, cache_path| {
        let (tx, rx) = mpsc::sync_channel(1);
        if let Err(e) = actor_ref.cast(ImageMsg::Encode {
            url,
            cache_path,
            done: tx,
        }) {
            eprintln!("ImageActor cast failed: {}", e);
            return None;
        }
        Some(ImageTicket { done: rx })
    });
    register_encoder(encoder);
    Ok(())
}
