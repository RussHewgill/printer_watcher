use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use std::{result, sync::Arc};

use ffmpeg_next as ffmpeg;
use serde::{Deserialize, Serialize};

use processor::H264Processor;

#[derive(Debug, Clone, Copy)]
pub enum RtspCommand {
    SetSkipFrames(bool),
    ToggleSkipFrames,
    // Stop,
}

pub mod processor {
    use anyhow::{anyhow, bail, ensure, Context, Result};
    use ffmpeg_next as ffmpeg;
    use tracing::{debug, error, info, trace, warn};

    pub struct H264Processor {
        decoder: ffmpeg::codec::decoder::Video,
        scaler: Option<ffmpeg::software::scaling::Context>,
        frame_i: u64,
        convert_to_annex_b: bool,
        handle: egui::TextureHandle,
        ctx: egui::Context,

        pub skip_non_raps: bool,
    }

    impl H264Processor {
        pub fn new(
            // convert_to_annex_b: bool
            handle: egui::TextureHandle,
            ctx: egui::Context,
            skip_non_raps: bool,
        ) -> Self {
            let convert_to_annex_b = false;

            let mut codec_opts = ffmpeg::Dictionary::new();
            if !convert_to_annex_b {
                codec_opts.set("is_avc", "1");
            }
            let codec = ffmpeg::codec::decoder::find(ffmpeg::codec::Id::H264).unwrap();
            let decoder = ffmpeg::codec::decoder::Decoder(ffmpeg::codec::Context::new())
                .open_as_with(codec, codec_opts)
                .unwrap()
                .video()
                .unwrap();
            Self {
                decoder,
                scaler: None,
                frame_i: 0,
                convert_to_annex_b,
                handle,
                ctx,
                skip_non_raps,
            }
        }

        pub fn handle_parameters(
            &mut self,
            // stream: &retina::client::Stream,
            p: &retina::codec::VideoParameters,
        ) -> Result<()> {
            if !self.convert_to_annex_b {
                let pkt = ffmpeg::codec::packet::Packet::borrow(p.extra_data());
                self.decoder.send_packet(&pkt)?;
            } else {
                // TODO: should convert and supply SPS/PPS, rather than relying on
                // them existing in-band within frames.
            }

            // let mut extra = p.extra_data().to_vec();
            // // debug!("extra data: {:?}", extra.len());
            // let (_, sps, pps) = super::decode_avc_decoder_config(&extra)?;

            // let mut extra = vec![];
            // extra.extend_from_slice(&[0x00, 0x00, 0x00, 0x01]);
            // extra.extend_from_slice(&sps);

            // extra.extend_from_slice(&[0x00, 0x00, 0x00, 0x01]);
            // extra.extend_from_slice(&pps);

            // self.decoder.set_parameters()
            // (*self.decoder.as_mut_ptr()).extradata

            // ffmpeg doesn't appear to actually handle the parameters until the
            // first full frame, so just note that the scaler needs to be
            // (re)created.
            self.scaler = None;
            Ok(())
        }

        pub fn send_frame(
            &mut self,
            // stream: &retina::client::Stream,
            f: retina::codec::VideoFrame,
        ) -> Result<()> {
            if self.skip_non_raps && !f.is_random_access_point() {
                return Ok(());
            }
            // let data = convert_h264(f)?;
            let data = if self.convert_to_annex_b {
                convert_h264(f)?
            } else {
                f.into_data()
            };
            let pkt = ffmpeg::codec::packet::Packet::borrow(&data);
            self.decoder.send_packet(&pkt)?;
            self.receive_frames()?;
            self.ctx.request_repaint();
            Ok(())
        }

        pub fn flush(&mut self) -> Result<()> {
            self.decoder.send_eof()?;
            self.receive_frames()?;
            Ok(())
        }

        fn receive_frames(&mut self) -> Result<()> {
            let mut decoded = ffmpeg::util::frame::video::Video::empty();
            loop {
                match self.decoder.receive_frame(&mut decoded) {
                    Err(ffmpeg::Error::Other {
                        errno: ffmpeg::util::error::EAGAIN,
                    }) => {
                        // No complete frame available.
                        break;
                    }
                    Err(e) => bail!(e),
                    Ok(()) => {}
                }

                // This frame writing logic lifted from ffmpeg-next's examples/dump-frames.rs.
                let scaler = self.scaler.get_or_insert_with(|| {
                    // info!(
                    //     "image parameters: {:?}, {}x{}",
                    //     self.decoder.format(),
                    //     self.decoder.width(),
                    //     self.decoder.height()
                    // );
                    ffmpeg::software::scaling::Context::get(
                        self.decoder.format(),
                        self.decoder.width(),
                        self.decoder.height(),
                        ffmpeg::format::Pixel::RGB24,
                        self.decoder.width(),
                        self.decoder.height(),
                        ffmpeg::software::scaling::Flags::BILINEAR,
                    )
                    .unwrap()
                });
                let mut scaled = ffmpeg::util::frame::video::Video::empty();
                scaler.run(&decoded, &mut scaled)?;

                // let image = image::load_from_memory(&scaled.data(0))?;
                let image = image::ImageBuffer::<image::Rgb<u8>, Vec<u8>>::from_raw(
                    scaled.width(),
                    scaled.height(),
                    scaled.data(0).to_vec(),
                )
                .unwrap();

                // // let filename = format!("frame_test.jpg");
                // let filename = format!("frame{}.jpg", self.frame_i);
                // image.save(filename)?;

                let img_size = [image.width() as _, image.height() as _];
                let pixels = image.as_flat_samples();
                let img = egui::ColorImage::from_rgb(img_size, pixels.as_slice());

                // let image: image::DynamicImage = image.into();
                // let img_size = [image.width() as _, image.height() as _];
                // let image_buffer = image.to_rgba8();
                // let pixels = image_buffer.as_flat_samples();
                // // let pixels = image.as_flat_samples();
                // // let img = egui::ColorImage::from_rgba_unmultiplied(img_size, pixels.as_slice());

                self.handle.set(img, egui::TextureOptions::default());

                self.frame_i += 1;
            }

            Ok(())
        }
    }

    /// https://github.com/scottlamb/retina/blob/main/examples/webrtc-proxy/src/main.rs#L310C1-L339C2
    fn convert_h264(
        // stream: &retina::client::Stream,
        frame: retina::codec::VideoFrame,
    ) -> Result<Vec<u8>> {
        // TODO:
        // * For each IDR frame, copy the SPS and PPS from the stream's
        //   parameters, rather than depend on it being present in the frame
        //   already. In-band parameters aren't guaranteed.

        let mut data = frame.into_data();
        let mut i = 0;
        while i < data.len() - 3 {
            // Replace each NAL's length with the Annex B start code b"\x00\x00\x00\x01".
            let len = u32::from_be_bytes([data[i], data[i + 1], data[i + 2], data[i + 3]]) as usize;
            data[i] = 0;
            data[i + 1] = 0;
            data[i + 2] = 0;
            data[i + 3] = 1;
            i += 4 + len;
            if i > data.len() {
                bail!("partial NAL body");
            }
        }
        if i < data.len() {
            bail!("partial NAL length");
        }
        Ok(data)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RtspCreds {
    pub host: String,
    pub username: String,
    pub password: String,
}

pub async fn rtsp_task(
    creds: RtspCreds,
    texture: egui::TextureHandle,
    kill_rx: tokio::sync::mpsc::UnboundedReceiver<()>,
    worker_cmd_rx: tokio::sync::mpsc::UnboundedReceiver<super::SubStreamCmd>,
    ctx: &egui::Context,
) -> Result<()> {
    /// Init ffmpeg
    ffmpeg_next::init().unwrap();

    if cfg!(debug_assertions) {
        // ffmpeg_next::util::log::set_level(ffmpeg_next::util::log::Level::Trace);
        ffmpeg_next::util::log::set_level(ffmpeg_next::util::log::Level::Warning);
    } else {
        ffmpeg_next::util::log::set_level(ffmpeg_next::util::log::Level::Trace);
    }

    let url = url::Url::parse(&format!("rtsp://{}", creds.host))?;

    let creds: retina::client::Credentials = retina::client::Credentials {
        username: creds.username.to_string(),
        password: creds.password.to_string(),
    };

    warn!("starting session");
    let session_group = Arc::new(retina::client::SessionGroup::default());
    let mut session = retina::client::Session::describe(
        url,
        retina::client::SessionOptions::default()
            .creds(Some(creds))
            .session_group(session_group.clone())
            .user_agent("printer_watcher".to_owned())
            .teardown(retina::client::TeardownPolicy::Auto), // XXX: auto?
    )
    .await?;

    warn!("getting index");
    let video_stream_i = {
        let s = session.streams().iter().position(|s| {
            if s.media() == "video" {
                if s.encoding_name() == "h264" {
                    info!("Using h264 video stream");
                    return true;
                }
                info!(
                    "Ignoring {} video stream because it's unsupported",
                    s.encoding_name(),
                );
            }
            false
        });
        match s {
            None => {
                warn!("No suitable video stream found");
                bail!("No suitable video stream found");
            }
            Some(s) => s,
        }
    };
    warn!("video stream index: {}", video_stream_i);

    session
        .setup(
            video_stream_i,
            retina::client::SetupOptions::default().transport(retina::client::Transport::Udp(
                retina::client::UdpTransportOptions::default(),
            )),
        )
        .await?;
    warn!("setup done");

    // let result = rtsp_loop(
    //     session,
    //     video_stream_i,
    //     texture,
    //     kill_rx,
    //     worker_cmd_rx,
    //     ctx,
    // )
    // .await;

    let mut worker = RtspWorker {
        video_stream_i,
        handle: texture,
        kill_rx,
        worker_cmd_rx,
        ctx: ctx.clone(),
    };

    let result = worker.rtsp_loop(session).await;

    if let Err(e) = session_group.await_teardown().await {
        error!("TEARDOWN failed: {}", e);
    }

    result
}

struct RtspWorker {
    video_stream_i: usize,
    handle: egui::TextureHandle,
    kill_rx: tokio::sync::mpsc::UnboundedReceiver<()>,
    worker_cmd_rx: tokio::sync::mpsc::UnboundedReceiver<super::SubStreamCmd>,
    ctx: egui::Context,
}

impl RtspWorker {
    async fn rtsp_loop(
        &mut self,
        session: retina::client::Session<retina::client::Described>,
    ) -> Result<()> {
        debug!("starting rtsp loop");

        let mut session = session
            .play(retina::client::PlayOptions::default())
            .await?
            .demuxed()?;

        let skip_non_raps = true;
        // let skip_non_raps = false;
        let mut processor =
            H264Processor::new(self.handle.clone(), self.ctx.clone(), skip_non_raps);

        if let Some(retina::codec::ParametersRef::Video(v)) =
            session.streams()[self.video_stream_i].parameters()
        {
            // debug!("initial parameters: {:#?}", v);
            processor.handle_parameters(v)?;
        }

        loop {
            let f = tokio::select! {
                _ = self.kill_rx.recv() => {
                    debug!("kill signal received");
                    break;
                }
                msg = self.worker_cmd_rx.recv() => {
                    if let Some(msg) = msg {
                        self.handle_msg(&mut processor, msg).await?;
                    }
                    continue;
                }
                f = futures::StreamExt::next(&mut session) => match f {
                    Some(Ok(f)) => f,
                    Some(Err(e)) => {
                        error!("error reading frame: {}", e);
                        continue;
                    }
                    None => {
                        info!("end of stream");
                        break;
                    }
                }

            };

            match f {
                retina::codec::CodecItem::VideoFrame(f) => {
                    let stream = &session.streams()[f.stream_id()];
                    let start_ctx = *f.start_ctx();
                    if f.has_new_parameters() {
                        let v = match stream.parameters() {
                            Some(retina::codec::ParametersRef::Video(v)) => {
                                debug!("new parameters: {:#?}", v);
                                v
                            }
                            _ => unreachable!(),
                        };
                        processor.handle_parameters(v)?;
                    }
                    processor.send_frame(f)?;
                    // break;
                }
                retina::codec::CodecItem::MessageFrame(msg) => {
                    info!("message: {:?}", msg);
                }
                retina::codec::CodecItem::AudioFrame(_) => {
                    // ignore
                }
                retina::codec::CodecItem::Rtcp(x) => {
                    // ignore
                }
                f => warn!("unexpected item: {:?}", f),
            }
        }

        processor.flush()?;
        Ok(())
    }

    async fn handle_msg(
        &mut self,
        processor: &mut H264Processor,
        msg: super::SubStreamCmd,
    ) -> Result<()> {
        match msg {
            super::SubStreamCmd::Rtsp(cmd) => match cmd {
                RtspCommand::SetSkipFrames(bool) => processor.skip_non_raps = bool,
                RtspCommand::ToggleSkipFrames => {
                    debug!("setting skip_non_raps to: {}", !processor.skip_non_raps);
                    processor.skip_non_raps = !processor.skip_non_raps;
                }
            },
        }
        Ok(())
    }
}

#[cfg(feature = "nope")]
async fn rtsp_loop(
    session: retina::client::Session<retina::client::Described>,
    video_stream_i: usize,
    handle: egui::TextureHandle,
    kill_rx: tokio::sync::mpsc::Receiver<()>,
    mut worker_cmd_rx: tokio::sync::mpsc::UnboundedReceiver<super::SubStreamCmd>,
    ctx: &egui::Context,
) -> Result<()> {
    debug!("starting rtsp loop");

    let mut session = session
        .play(retina::client::PlayOptions::default())
        .await?
        .demuxed()?;

    let skip_non_raps = true;
    let mut processor = H264Processor::new(handle, ctx.clone(), skip_non_raps);

    if let Some(retina::codec::ParametersRef::Video(v)) =
        session.streams()[video_stream_i].parameters()
    {
        debug!("initial parameters: {:#?}", v);
        processor.handle_parameters(v)?;
    }

    loop {
        let f = tokio::select! {
            msg = worker_cmd_rx.recv() => {
                continue;
            }
            f = futures::StreamExt::next(&mut session) => match f {
                Some(Ok(f)) => f,
                Some(Err(e)) => {
                    error!("error reading frame: {}", e);
                    continue;
                }
                None => {
                    info!("end of stream");
                    break;
                }
            }

        };

        match f {
            retina::codec::CodecItem::VideoFrame(f) => {
                let stream = &session.streams()[f.stream_id()];
                let start_ctx = *f.start_ctx();
                if f.has_new_parameters() {
                    let v = match stream.parameters() {
                        Some(retina::codec::ParametersRef::Video(v)) => {
                            debug!("new parameters: {:#?}", v);
                            v
                        }
                        _ => unreachable!(),
                    };
                    processor.handle_parameters(v)?;
                }
                processor.send_frame(f)?;
                // break;
            }
            retina::codec::CodecItem::MessageFrame(msg) => {
                info!("message: {:?}", msg);
            }
            retina::codec::CodecItem::AudioFrame(_) => {
                // ignore
            }
            retina::codec::CodecItem::Rtcp(x) => {
                // ignore
            }
            f => warn!("unexpected item: {:?}", f),
        }
    }

    #[cfg(feature = "nope")]
    loop {
        let f = match futures::StreamExt::next(&mut session).await {
            Some(Ok(f)) => f,
            Some(Err(e)) => {
                error!("error reading frame: {}", e);
                continue;
            }
            None => {
                info!("end of stream");
                break;
            }
        };

        match f {
            retina::codec::CodecItem::VideoFrame(f) => {
                let stream = &session.streams()[f.stream_id()];
                let start_ctx = *f.start_ctx();
                if f.has_new_parameters() {
                    let v = match stream.parameters() {
                        Some(retina::codec::ParametersRef::Video(v)) => {
                            debug!("new parameters: {:#?}", v);
                            v
                        }
                        _ => unreachable!(),
                    };
                    processor.handle_parameters(v)?;
                }
                processor.send_frame(f)?;
                // break;
            }
            retina::codec::CodecItem::MessageFrame(msg) => {
                info!("message: {:?}", msg);
            }
            retina::codec::CodecItem::AudioFrame(_) => {
                // ignore
            }
            retina::codec::CodecItem::Rtcp(x) => {
                // ignore
            }
            f => warn!("unexpected item: {:?}", f),
        }
    }

    processor.flush()?;
    Ok(())
}
