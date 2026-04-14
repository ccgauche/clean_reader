use ractor::{Actor, ActorProcessingErr, ActorRef};
use reader_core::pipeline;

use crate::message::PageMsg;

/// ractor actor wrapping [`reader_core::pipeline::render`]. Each render
/// message is dispatched to its own tokio task so concurrent renders don't
/// serialize on the mailbox.
pub struct PageActor;

impl Actor for PageActor {
    type Msg = PageMsg;
    type State = ();
    type Arguments = ();

    async fn pre_start(
        &self,
        _myself: ActorRef<PageMsg>,
        _args: (),
    ) -> std::result::Result<(), ActorProcessingErr> {
        Ok(())
    }

    async fn handle(
        &self,
        _myself: ActorRef<PageMsg>,
        msg: PageMsg,
        _state: &mut (),
    ) -> std::result::Result<(), ActorProcessingErr> {
        match msg {
            PageMsg::Render {
                url,
                min_id,
                mode,
                reply,
            } => {
                // Fan out: each render runs on its own tokio task so the
                // mailbox drains fast and concurrent renders don't
                // serialize behind each other.
                tokio::spawn(async move {
                    let result = pipeline::render(&url, &min_id, mode).await;
                    let _ = reply.send(result);
                });
            }
        }
        Ok(())
    }
}
