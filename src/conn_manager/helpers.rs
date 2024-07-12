use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use crate::{
    config::{printer_config::PrinterConfig, printer_id::PrinterId},
    status::GenericPrinterState,
};

use super::worker_message::WorkerMsg;

pub async fn spawn_fetch_thumbnail(
    printer: PrinterConfig,
    state: GenericPrinterState,
    id: PrinterId,
    file: String,
    // file: String,
    worker_msg_tx: tokio::sync::mpsc::UnboundedSender<(PrinterId, WorkerMsg)>,
) -> Result<()> {
    match printer {
        PrinterConfig::Bambu(_, _) => todo!(),
        PrinterConfig::Klipper(_, _) => todo!(),
        // PrinterConfig::Octoprint(_, _) => todo!(),
        PrinterConfig::Prusa(_, printer) => {
            let printer = printer.read().await;

            let host = printer.host.clone();
            let key = printer.key.clone();

            let Some(state) = state.state_prusa else {
                bail!("Printer state is not Prusa");
            };
            let thumbnail = state.job.file.refs.thumbnail.clone();

            tokio::spawn(async move {
                debug!("spawned fetch thumbnail task");
                let client = reqwest::ClientBuilder::new().build().unwrap();

                let url = format!("http://{}{}", host, thumbnail);

                debug!("sending request");
                let resp = client
                    .get(&url)
                    .header("X-Api-Key", &key)
                    // .send()
                    .send()
                    .await
                    .unwrap();

                debug!("got response, status = {:?}", resp.status());

                let bytes = resp.bytes().await.unwrap();

                debug!("got bytes");

                worker_msg_tx
                    .send((
                        id.clone(),
                        WorkerMsg::FetchedThumbnail(id, file, bytes.to_vec()),
                    ))
                    .unwrap();
            });

            Ok(())
        }
    }
}
