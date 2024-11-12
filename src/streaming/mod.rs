pub mod bambu;
#[cfg(feature = "rtsp")]
pub mod rtsp;

use core::error;
use std::collections::HashMap;

use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

#[cfg(feature = "rtsp")]
use ffmpeg_next::codec::debug;
#[cfg(feature = "rtsp")]
use rtsp::{RtspCommand, RtspCreds};

use egui::TextureHandle;

use crate::config::printer_id::PrinterId;

#[derive(Clone)]
pub enum StreamCmd {
    #[cfg(feature = "rtsp")]
    StartRtsp(PrinterId, TextureHandle, RtspCreds, egui::Context),
    StartBambuStills {
        id: PrinterId,
        host: String,
        access_code: String,
        serial: String,
        texture: TextureHandle,
    },
    StopStream(PrinterId),
    SendRtspCommand(PrinterId, SubStreamCmd),
}

#[derive(Debug, Clone, Copy)]
pub enum SubStreamCmd {
    #[cfg(feature = "rtsp")]
    Rtsp(RtspCommand),
}

#[derive(Clone)]
pub enum StreamWorkerMsg {
    Panic(PrinterId, StreamCmd),
}

pub struct StreamManager {
    cmd_tx: tokio::sync::mpsc::UnboundedSender<StreamCmd>,
    cmd_rx: tokio::sync::mpsc::UnboundedReceiver<StreamCmd>,

    worker_tx: tokio::sync::mpsc::UnboundedSender<StreamWorkerMsg>,
    worker_rx: tokio::sync::mpsc::UnboundedReceiver<StreamWorkerMsg>,

    worker_channels: HashMap<
        PrinterId,
        (
            tokio::sync::mpsc::UnboundedSender<()>,
            tokio::sync::mpsc::UnboundedSender<SubStreamCmd>,
        ),
    >,
}

impl StreamManager {
    pub fn new(
        cmd_tx: tokio::sync::mpsc::UnboundedSender<StreamCmd>,
        cmd_rx: tokio::sync::mpsc::UnboundedReceiver<StreamCmd>,
        // cmd_rx: tokio::sync::mpsc::UnboundedReceiver<StreamCmd>,
    ) -> Self {
        let (worker_tx, worker_rx) = tokio::sync::mpsc::unbounded_channel();

        Self {
            cmd_tx,
            cmd_rx,
            worker_tx,
            worker_rx,
            worker_channels: HashMap::new(),
        }
    }

    pub async fn run(&mut self) -> Result<()> {
        loop {
            tokio::select! {
                msg = self.worker_rx.recv() => {
                    match msg {
                        None => {},
                        Some(StreamWorkerMsg::Panic(id, cmd)) => {
                            error!("stream panic for printer: {:?}, restarting", id);
                            self.cmd_tx.send(cmd).unwrap();
                        }
                    }
                }
                cmd = self.cmd_rx.recv() => match cmd {
                    None => return Ok(()),
                    #[cfg(feature = "rtsp")]
                    Some(StreamCmd::StartRtsp(id, texture_handle, creds, ctx)) => {
                        debug!("starting RTSP stream for printer: {:?}", id);
                        self.start_stream_rtsp(id, texture_handle, creds, ctx, self.worker_tx.clone()).await?;
                    }
                    Some(StreamCmd::StartBambuStills { id, host, access_code, serial, texture }) => {
                        debug!("starting Bambu still stream");
                        self.start_stream_bambu_stills(id, host, access_code, serial, texture, self.worker_tx.clone()).await?;
                    }
                    Some(StreamCmd::SendRtspCommand(id, cmd)) => {
                        // debug!("sending RTSP command");
                        if let Some((_, tx)) = self.worker_channels.get(&id) {
                            tx.send(cmd).unwrap();
                        }
                        // self.rtsp_tx.send(cmd).unwrap();
                    }
                    Some(StreamCmd::StopStream(id)) => {
                        debug!("stopping stream for printer: {:?}", id);
                        if let Some((tx, _)) = self.worker_channels.remove(&id) {
                            tx.send(()).unwrap();
                        }
                    }
                }
            }
        }
    }

    async fn start_stream_bambu_stills(
        &mut self,
        id: PrinterId,
        host: String,
        access_code: String,
        serial: String,
        texture: egui::TextureHandle,
        worker_tx: tokio::sync::mpsc::UnboundedSender<StreamWorkerMsg>,
    ) -> Result<()> {
        let (kill_tx, kill_rx) = tokio::sync::mpsc::unbounded_channel::<()>();

        tokio::spawn(async move {
            let cmd = StreamCmd::StartBambuStills {
                id: id.clone(),
                host: host.clone(),
                access_code: access_code.clone(),
                serial: serial.clone(),
                texture: texture.clone(),
            };

            let mut conn = match bambu::bambu_img::JpegStreamViewer::new(
                id.clone(),
                serial,
                host,
                access_code,
                texture,
                kill_rx,
            )
            .await
            {
                Ok(c) => c,
                Err(e) => {
                    error!("error creating bambu stills: {:?}", e);
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                    worker_tx
                        .send(StreamWorkerMsg::Panic(id.clone(), cmd))
                        .unwrap();
                    return;
                }
            };

            if let Err(e) = conn.run().await {
                error!("error in bambu stills: {:?}", e);
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                worker_tx
                    .send(StreamWorkerMsg::Panic(id.clone(), cmd))
                    .unwrap();
            }
        });

        Ok(())
    }

    #[cfg(feature = "rtsp")]
    async fn start_stream_rtsp(
        &mut self,
        id: PrinterId,
        texture_handle: TextureHandle,
        creds: RtspCreds,
        ctx: egui::Context,
        worker_tx: tokio::sync::mpsc::UnboundedSender<StreamWorkerMsg>,
        // rtsp_tx: tokio::sync::mpsc::UnboundedSender<RtspCommand>,
    ) -> Result<()> {
        let (kill_tx, kill_rx) = tokio::sync::mpsc::unbounded_channel::<()>();
        let worker_tx = self.worker_tx.clone();
        // tokio::spawn(async move {
        //     crate::streaming::rtsp::rtsp_task(creds, texture_handle, kill_rx)
        //         .await
        //         .unwrap();
        //     //
        // });

        let (worker_cmd_tx, worker_cmd_rx) = tokio::sync::mpsc::unbounded_channel();

        self.worker_channels
            .insert(id.clone(), (kill_tx, worker_cmd_tx));

        /// ffmpeg doesn't work across tasks
        std::thread::spawn(|| {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async move {
                if let Err(e) = crate::streaming::rtsp::rtsp_task(
                    creds,
                    texture_handle,
                    kill_rx,
                    worker_cmd_rx,
                    &ctx,
                )
                .await
                {
                    error!("error in rtsp: {:?}", e);
                }
            })
        });
        Ok(())
    }
}
