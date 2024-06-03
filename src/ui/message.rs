use std::sync::Arc;

use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use crate::conn_manager::PrinterConnMsg;

#[derive(Debug, Clone)]
pub enum AppMsg {
    PrinterConnMsg(PrinterConnMsg),
    //
}

// enum State {
//     Starting,
//     Ready(mpsc::Receiver<PrinterConnMsg>),
// }

// fn some_worker() -> Subscription<Event> {
//     struct SomeWorker;

pub fn subscribe(
    rx: &Arc<std::sync::Mutex<tokio::sync::mpsc::UnboundedReceiver<PrinterConnMsg>>>,
) -> iced::Subscription<AppMsg> {
    debug!("subscribing to printer connection messages");
    // iced::Subscription::from_recipe(subscribe())

    struct Connect;

    // subscription::channel(
    //     std::any::TypeId::of::<Connect>(),
    //     100,
    //     |mut output| async move {
    //         loop {
    //             //
    //         }
    //     },
    // )
    unimplemented!()
}

#[cfg(feature = "nope")]
pub fn subscribe(
    mut rx: tokio::sync::mpsc::UnboundedReceiver<PrinterConnMsg>,
) -> iced::Subscription<AppMsg> {
    debug!("subscribing to printer connection messages");
    iced::subscription::channel(
        std::any::TypeId::of::<PrinterConnMsg>(),
        25,
        move |mut sender| async move {
            loop {
                debug!("looping");
                let msg = rx.recv().await.unwrap();
                // debug!("received msg: {:?}", msg);
                if let Err(e) =
                    futures::SinkExt::send(&mut sender, AppMsg::PrinterConnMsg(msg)).await
                {
                    error!("error sending message: {:?}", e);
                }
            }
        },
    )
}

#[cfg(feature = "nope")]
pub struct AppMsgRecipe {
    pub rx: tokio::sync::mpsc::UnboundedReceiver<PrinterConnMsg>,
}

#[cfg(feature = "nope")]
impl iced::advanced::subscription::Recipe for AppMsgRecipe {
    type Output = AppMsg;

    fn hash(&self, state: &mut iced::advanced::Hasher) {
        use std::hash::Hash;

        std::any::TypeId::of::<Self>().hash(state);
        "AppMsgRecipe".hash(state);
    }

    fn stream(
        mut self: Box<Self>,
        input: iced::advanced::subscription::EventStream,
    ) -> futures::stream::BoxStream<'static, Self::Output> {
        use ::futures::{FutureExt, StreamExt};
        // self.rx.
        // let mut rx = ;
        // Box::pin(
        //     rx.recv()
        //         .into_stream()
        //         .map(|msg| AppMsg::PrinterConnMsg(msg.unwrap())),
        // )
        let stream = async_stream::stream! {
            while let Some(item) = self.rx.recv().await {
                yield item;
            }
        };

        // let stream = futures::stream::unfold((), )
        stream.map(|msg| AppMsg::PrinterConnMsg(msg)).boxed()
    }
}
