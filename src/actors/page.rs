use ractor::{Actor, ActorProcessingErr, ActorRef, RpcReplyPort};

use crate::error::Result;
use crate::pipeline::run_v2;

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
                // mailbox drains fast and concurrent renders don't serialize.
                tokio::spawn(async move {
                    let result = run_v2(&url, &min_id, as_download).await;
                    if reply.send(result).is_err() {
                        // Caller dropped the future. Nothing actionable here.
                    }
                });
            }
        }
        Ok(())
    }
}
