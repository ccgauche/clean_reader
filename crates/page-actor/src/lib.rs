//! Page-rendering actor.
//!
//! Wraps [`reader_core::pipeline::run_v2`] in a ractor actor. Each render
//! message is dispatched to its own tokio task so concurrent renders don't
//! serialize on the mailbox.

use once_cell::sync::OnceCell;
use ractor::concurrency::JoinHandle;
use ractor::rpc::CallResult;
use ractor::{Actor, ActorProcessingErr, ActorRef, RpcReplyPort};
use reader_core::error::{Error, Result};
use reader_core::pipeline::run_v2;

pub enum PageMsg {
    Render {
        url: String,
        min_id: String,
        as_download: bool,
        reply: RpcReplyPort<Result<String>>,
    },
}

pub struct PageActor;
pub struct PageState;

impl Actor for PageActor {
    type Msg = PageMsg;
    type State = PageState;
    type Arguments = ();

    async fn pre_start(
        &self,
        _myself: ActorRef<PageMsg>,
        _args: (),
    ) -> std::result::Result<PageState, ActorProcessingErr> {
        Ok(PageState)
    }

    async fn handle(
        &self,
        _myself: ActorRef<PageMsg>,
        msg: PageMsg,
        _state: &mut PageState,
    ) -> std::result::Result<(), ActorProcessingErr> {
        match msg {
            PageMsg::Render {
                url,
                min_id,
                as_download,
                reply,
            } => {
                // Fan out: each render runs on its own tokio task so the
                // mailbox drains fast and concurrent renders don't
                // serialize.
                tokio::spawn(async move {
                    let result = run_v2(&url, &min_id, as_download).await;
                    let _ = reply.send(result);
                });
            }
        }
        Ok(())
    }
}

static PAGE_REF: OnceCell<ActorRef<PageMsg>> = OnceCell::new();
static ACTOR_HANDLE: OnceCell<JoinHandle<()>> = OnceCell::new();

/// Spawn the page actor and stash the actor ref for later `render_page`
/// calls.
pub async fn boot() -> Result<()> {
    let (actor_ref, handle): (ActorRef<PageMsg>, JoinHandle<()>) =
        Actor::spawn(Some("page".into()), PageActor, ())
            .await
            .map_err(|e| Error::Actor(format!("spawn PageActor: {}", e)))?;
    PAGE_REF
        .set(actor_ref)
        .map_err(|_| Error::Actor("PageActor already booted".into()))?;
    let _ = ACTOR_HANDLE.set(handle);
    Ok(())
}

/// Ask the page actor to render a URL.
pub async fn render_page(url: &str, min_id: &str, as_download: bool) -> Result<String> {
    let actor = PAGE_REF
        .get()
        .ok_or_else(|| Error::Actor("PageActor not booted".into()))?;
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
        .map_err(|e| Error::Actor(format!("PageActor call: {}", e)))?;
    match result {
        CallResult::Success(r) => r,
        CallResult::Timeout => Err(Error::Actor("PageActor timeout".into())),
        CallResult::SenderError => Err(Error::Actor("PageActor reply dropped".into())),
    }
}
