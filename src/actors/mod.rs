//! Actor layer.
//!
//! Two singleton actors fan work out of the request path:
//!
//! - `PageActor` owns the article-clean pipeline. Each `Render` message is
//!   dispatched to its own tokio task, so concurrent renders don't serialize
//!   on the mailbox.
//! - `ImageActor` owns the image re-encode pipeline. Each `Encode` message is
//!   dispatched to its own OS thread (ravif is CPU-heavy + rayon-parallel).
//!
//! Both refs live in `OnceCell`s set by `init()` at server startup. Callers
//! interact through the public helpers in this module rather than touching
//! ractor types directly.

use std::path::PathBuf;
use std::sync::mpsc;
use std::time::Duration;

use once_cell::sync::OnceCell;
use ractor::{concurrency::JoinHandle, Actor, ActorRef};

use crate::error::{Error, Result};
use crate::image::ImageTicket;

pub mod image;
pub mod page;

use image::{ImageActor, ImageMsg};
use page::{PageActor, PageMsg};

static PAGE_REF: OnceCell<ActorRef<PageMsg>> = OnceCell::new();
static IMAGE_REF: OnceCell<ActorRef<ImageMsg>> = OnceCell::new();

// Keep the actor join handles alive for the lifetime of the process.
static ACTOR_HANDLES: OnceCell<Vec<JoinHandle<()>>> = OnceCell::new();

/// Spawn both actors. Must be called once, inside a tokio runtime, before
/// any handler tries to use them.
pub async fn init() -> Result<()> {
    let (page_ref, page_handle) = Actor::spawn(Some("page".into()), PageActor, ())
        .await
        .map_err(|e| Error::Render(format!("spawn PageActor: {}", e)))?;
    let (image_ref, image_handle) = Actor::spawn(Some("image".into()), ImageActor, ())
        .await
        .map_err(|e| Error::Render(format!("spawn ImageActor: {}", e)))?;

    PAGE_REF
        .set(page_ref)
        .map_err(|_| Error::Render("PageActor already initialised".into()))?;
    IMAGE_REF
        .set(image_ref)
        .map_err(|_| Error::Render("ImageActor already initialised".into()))?;
    ACTOR_HANDLES
        .set(vec![page_handle, image_handle])
        .map_err(|_| Error::Render("actor handles already set".into()))?;
    Ok(())
}

/// Ask the PageActor to render a URL. Returns the rendered HTML or the
/// pipeline error.
pub async fn render_page(url: &str, min_id: &str, as_download: bool) -> Result<String> {
    let actor = PAGE_REF
        .get()
        .ok_or_else(|| Error::Render("PageActor not initialised".into()))?;
    let url = url.to_owned();
    let min_id = min_id.to_owned();
    let result = actor
        .call(
            |reply| PageMsg::Render {
                url,
                min_id,
                as_download,
                reply,
            },
            None,
        )
        .await
        .map_err(|e| Error::Render(format!("PageActor call: {}", e)))?;
    match result {
        ractor::rpc::CallResult::Success(r) => r,
        ractor::rpc::CallResult::Timeout => Err(Error::Render("PageActor timeout".into())),
        ractor::rpc::CallResult::SenderError => {
            Err(Error::Render("PageActor reply dropped".into()))
        }
    }
}

/// Cast an image re-encode request to the ImageActor. Returns a ticket the
/// template renderer can block on to make sure the cached file lands on disk
/// before the response goes out, or `None` if the actor isn't up (shouldn't
/// happen after `init()` but we fall back to "no actor → no ticket" rather
/// than panicking from inside the template path).
pub fn encode_image(url: String, cache_path: PathBuf) -> Option<ImageTicket> {
    let actor = IMAGE_REF.get()?;
    let (done_tx, done_rx) = mpsc::sync_channel(1);
    if let Err(e) = actor.cast(ImageMsg::Encode {
        url,
        cache_path,
        done: done_tx,
    }) {
        eprintln!("ImageActor cast failed: {}", e);
        return None;
    }
    Some(ImageTicket { done: done_rx })
}

/// Wait on an image ticket, bounding the total wait by a reasonable timeout
/// so a stuck worker can't hold the response open forever.
pub fn wait_for_image(ticket: ImageTicket) {
    // 15s is generous for "fetch a JPEG and run ravif at speed=5". Anything
    // longer and the browser will have given up anyway.
    let _ = ticket.done.recv_timeout(Duration::from_secs(15));
}
