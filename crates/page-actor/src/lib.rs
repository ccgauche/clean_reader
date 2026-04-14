//! Page-rendering actor. See [`actor::PageActor`] for the work loop and
//! [`message::PageMsg`] for the message protocol.
//!
//! The public API of this crate is `boot()` + `render_page()`: the
//! server should never touch the underlying ractor types directly.

mod actor;
mod error;
mod message;

pub use error::PageActorError;
pub use message::PageMsg;

use once_cell::sync::OnceCell;
use ractor::concurrency::JoinHandle;
use ractor::rpc::CallResult;
use ractor::{Actor, ActorRef};
use reader_core::render_mode::RenderMode;

use actor::PageActor;

static PAGE_REF: OnceCell<ActorRef<PageMsg>> = OnceCell::new();
static ACTOR_HANDLE: OnceCell<JoinHandle<()>> = OnceCell::new();

/// Spawn the page actor and stash the actor ref for later `render_page`
/// calls. Must be invoked from inside a tokio runtime, exactly once per
/// process.
pub async fn boot() -> Result<(), PageActorError> {
    let (actor_ref, handle): (ActorRef<PageMsg>, JoinHandle<()>) =
        Actor::spawn(Some("page".into()), PageActor, ())
            .await
            .map_err(|e| PageActorError::SpawnFailed(e.to_string()))?;
    PAGE_REF
        .set(actor_ref)
        .map_err(|_| PageActorError::SpawnFailed("PageActor already booted".into()))?;
    let _ = ACTOR_HANDLE.set(handle);
    Ok(())
}

/// Ask the page actor to render a URL.
pub async fn render_page(
    url: &str,
    min_id: &str,
    mode: RenderMode,
) -> Result<String, PageActorError> {
    let actor = PAGE_REF.get().ok_or(PageActorError::NotBooted)?;
    let url = url.to_owned();
    let min_id = min_id.to_owned();
    let call = actor
        .call(
            |reply| PageMsg::Render {
                url,
                min_id,
                mode,
                reply,
            },
            None,
        )
        .await
        .map_err(|e| PageActorError::CallFailed(e.to_string()))?;
    match call {
        CallResult::Success(result) => result.map_err(PageActorError::Pipeline),
        CallResult::Timeout => Err(PageActorError::Timeout),
        CallResult::SenderError => Err(PageActorError::ReplyDropped),
    }
}
