//! Image re-encoding actor. See [`actor::ImageActor`] for the work loop
//! and [`message::ImageMsg`] for the message protocol.
//!
//! At boot, the crate spawns its actor and registers a closure with
//! [`reader_core::image::register_encoder`] so the template renderer can
//! trigger encoding without importing this crate.

mod actor;
mod error;
mod message;

pub use error::ImageActorError;
pub use message::ImageMsg;

use std::sync::mpsc;

use once_cell::sync::OnceCell;
use ractor::concurrency::JoinHandle;
use ractor::{Actor, ActorRef};
use reader_core::image::{register_encoder, EncoderFn, ImageTicket};

use actor::ImageActor;

/// Keep the actor JoinHandle alive for the lifetime of the process so
/// the actor isn't collected out from under us after `boot()` returns.
static ACTOR_HANDLE: OnceCell<JoinHandle<()>> = OnceCell::new();

/// Spawn the image actor and register its encoder with reader-core.
pub async fn boot() -> Result<(), ImageActorError> {
    let (actor_ref, handle): (ActorRef<ImageMsg>, JoinHandle<()>) =
        Actor::spawn(Some("image".into()), ImageActor, ())
            .await
            .map_err(|e| ImageActorError::SpawnFailed(e.to_string()))?;
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
