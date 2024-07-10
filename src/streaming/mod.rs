pub mod bambu;
pub mod rtsp;

use anyhow::{anyhow, bail, ensure, Context, Result};
use rtsp::RtspCreds;
use tracing::{debug, error, info, trace, warn};

use egui::TextureHandle;

use crate::config::printer_id::PrinterId;

#[derive(Clone)]
pub enum StreamCmd {
    StartRtsp(PrinterId, TextureHandle, RtspCreds, egui::Context),
    StartBambuStills {
        id: PrinterId,
        host: String,
        access_code: String,
        serial: String,
        texture: TextureHandle,
    },
}

#[derive(Clone)]
pub enum StreamWorkerMsg {
    Panic(PrinterId),
}

pub struct StreamManager {
    cmd_rx: tokio::sync::mpsc::UnboundedReceiver<StreamCmd>,

    worker_tx: tokio::sync::mpsc::UnboundedSender<StreamWorkerMsg>,
    worker_rx: tokio::sync::mpsc::UnboundedReceiver<StreamWorkerMsg>,
}

impl StreamManager {
    pub fn new(
        cmd_rx: tokio::sync::mpsc::UnboundedReceiver<StreamCmd>,
        // cmd_rx: tokio::sync::mpsc::UnboundedReceiver<StreamCmd>,
    ) -> Self {
        let (worker_tx, worker_rx) = tokio::sync::mpsc::unbounded_channel();
        Self {
            cmd_rx,
            worker_tx,
            worker_rx,
        }
    }

    pub async fn run(&mut self) -> Result<()> {
        loop {
            tokio::select! {
                msg = self.worker_rx.recv() => {
                    //
                }
                cmd = self.cmd_rx.recv() => match cmd {
                    None => return Ok(()),
                    Some(StreamCmd::StartRtsp(id, texture_handle, creds, ctx)) => {
                        debug!("starting RTSP stream for printer: {:?}", id);
                        self.start_stream_rtsp(id, texture_handle, creds, ctx).await?;
                    }
                    Some(StreamCmd::StartBambuStills { id, host, access_code, serial, texture }) => {
                        debug!("starting Bambu still stream");
                        self.start_stream_bambu_stills(id, host, access_code, serial, texture).await?;
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
    ) -> Result<()> {
        let (kill_tx, kill_rx) = tokio::sync::mpsc::channel::<()>(1);

        tokio::spawn(async move {
            let mut conn = bambu::bambu_img::JpegStreamViewer::new(
                id,
                serial,
                host,
                access_code,
                texture,
                kill_rx,
            )
            .await
            .unwrap();

            conn.run().await.unwrap();
        });

        Ok(())
    }

    async fn start_stream_rtsp(
        &mut self,
        id: PrinterId,
        texture_handle: TextureHandle,
        creds: RtspCreds,
        ctx: egui::Context,
    ) -> Result<()> {
        let (kill_tx, kill_rx) = tokio::sync::mpsc::channel::<()>(1);
        let worker_tx = self.worker_tx.clone();
        // tokio::spawn(async move {
        //     crate::streaming::rtsp::rtsp_task(creds, texture_handle, kill_rx)
        //         .await
        //         .unwrap();
        //     //
        // });

        /// ffmpeg doesn't work across tasks
        std::thread::spawn(|| {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async move {
                crate::streaming::rtsp::rtsp_task(creds, texture_handle, kill_rx, &ctx)
                    .await
                    .unwrap();
            })
        });
        Ok(())
    }
}

/// packets
#[cfg(feature = "nope")]
pub async fn write_frames(
    session: retina::client::Session<retina::client::Described>,
    stop_signal: std::pin::Pin<Box<dyn futures::Future<Output = Result<(), std::io::Error>>>>,
) -> Result<()> {
    use ffmpeg_next as ffmpeg;
    ffmpeg_next::init()?;

    tokio::pin!(stop_signal);

    let mut session = session.play(retina::client::PlayOptions::default()).await?;

    /// ffmpeg setup
    let codec: ffmpeg_next::Codec = ffmpeg::decoder::find(ffmpeg::codec::Id::H264).unwrap();

    let context_decoder = ffmpeg::codec::context::Context::new_with_codec(codec);
    debug!("getting decoder");
    let mut decoder: ffmpeg_next::decoder::Video = context_decoder.decoder().video().unwrap();
    debug!("got decoder");

    debug!("setting decoder params");
    /// SAFETY: unsure?
    unsafe {
        (*decoder.as_mut_ptr()).height = 1080;
        (*decoder.as_mut_ptr()).width = 1920;
        // XXX: hardcode for now?
        (*decoder.as_mut_ptr()).pix_fmt = ffmpeg::ffi::AVPixelFormat::AV_PIX_FMT_YUV420P;
    }

    let mut scaler = ffmpeg::software::scaling::context::Context::get(
        decoder.format(),
        decoder.width(),
        decoder.height(),
        ffmpeg_next::format::Pixel::RGB24,
        decoder.width(),
        decoder.height(),
        ffmpeg_next::software::scaling::Flags::BILINEAR,
    )?;

    debug!("getting packets");
    loop {
        match futures::StreamExt::next(&mut session).await {
            Some(Ok(retina::client::PacketItem::Rtp(pkt))) => {
                debug!("got RTP packet");
                // debug!("mark: {:?}", pkt.mark());
                // debug!("stream_id: {:?}", pkt.stream_id());
                // debug!("ctx: {:?}", pkt.ctx());
                // debug!("ssrc: {:?}", pkt.ssrc());
                // debug!("sequence_number: {:?}", pkt.sequence_number());
                // debug!("raw.len: {:?}", pkt.raw().len());
                // debug!("payload.len: {:?}", pkt.payload().len());
                // debug!("loss: {:?}", pkt.loss());

                let data = pkt.payload();

                debug!("frame len: {:?}", data.len());

                let packet = ffmpeg::Packet::copy(&data);

                decoder.send_packet(&packet)?;

                break;
                //
            }
            Some(Ok(retina::client::PacketItem::Rtcp(pkt))) => {
                warn!("unexpected RTCP packet");
            }
            _ => {
                bail!("unexpected item");
            }
        }
    }

    Ok(())
}
