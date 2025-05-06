use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use parking_lot::Mutex;
use std::{cell::LazyCell, io::Write, str::FromStr, sync::Arc};

use gst::prelude::*;
use gstreamer::{self as gst, glib::FlagsClass};
use gstreamer_app as gst_app;
use gstreamer_rtsp as gst_rtsp;
use gstreamer_video as gst_video;

use crate::config::printer_id::PrinterId;

use super::StreamCmd;

pub struct GStreamerPlayer {
    id: PrinterId,
    pub uri: String,
    texture_handle: egui::TextureHandle,
    // texture_handle: Arc<Mutex<Option<egui::TextureHandle>>>,
    // cmd_rx: crossbeam_channel::Receiver<crate::streaming::StreamCmd>,
    cmd_tx: tokio::sync::mpsc::UnboundedSender<super::StreamWorkerMsg>,
    // kill_rx: Option<tokio::sync::mpsc::UnboundedReceiver<()>>,
    start_time: std::time::Instant,
    panic_cmd: StreamCmd,
}

impl GStreamerPlayer {
    pub fn new(
        id: PrinterId,
        // username: &str,
        password: String,
        host: String,
        port: u16,
        serial: String,
        // port: u16,
        texture_handle: egui::TextureHandle,
        // cmd_rx: crossbeam_channel::Receiver<crate::streaming::StreamCmd>,
        cmd_tx: tokio::sync::mpsc::UnboundedSender<super::StreamWorkerMsg>,
        // kill_rx: tokio::sync::mpsc::UnboundedReceiver<()>,
    ) -> Self {
        let access_code = std::env::var("RTSP_PASS").unwrap();
        let uri = format!(
            // "rtsps://bblp:{}@192.168.0.23:322/streaming/live/1",
            "rtsps://bblp:{}@{}:{}/streaming/live/1",
            access_code, host, port
        );

        let panic_cmd = StreamCmd::StartRtsp {
            id: id.clone(),
            host: host.to_string(),
            access_code: password.to_string(),
            serial,
            texture: texture_handle.clone(),
        };

        Self {
            id,
            uri,
            texture_handle,
            cmd_tx,
            // kill_rx: Some(kill_rx),
            start_time: std::time::Instant::now(),
            panic_cmd,
        }
    }

    pub fn init(
        &mut self,
        kill_rx: tokio::sync::mpsc::UnboundedReceiver<()>,
        cmd_rx: tokio::sync::mpsc::UnboundedReceiver<super::SubStreamCmd>,
    ) -> Result<()> {
        let worker_tx = self.cmd_tx.clone();
        run_gstreamer(
            self.id.clone(),
            // (276, 155),
            (1710, 960),
            &self.start_time,
            &self.uri,
            self.texture_handle.clone(),
            kill_rx,
            cmd_rx,
            worker_tx,
            self.panic_cmd.clone(),
        )?;
        Ok(())
    }
}

struct PipelineData {
    pipeline: gst::Pipeline,
    appsink: gst_app::AppSink,
    // frame_buffer: Arc<Mutex<Option<Vec<u8>>>>,
    // texture_handle: Arc<Mutex<Option<egui::TextureHandle>>>,
    texture_handle: egui::TextureHandle,
    // Keep track of width/height/format once known
    frame_info: Arc<Mutex<Option<gst_video::VideoInfo>>>,
}

#[derive(Clone, Debug)]
struct DiscoveredStream {
    stream_id: String,
    caps: gst::Caps,
    width: u32,
    height: u32,
}

#[derive(Clone, Debug)]
struct SelectedStream {
    stream_id: String,
    width: u32,
    height: u32,
}

fn build_pipeline(
    desired_res: (u32, u32),
    uri: &str,
    // frame_buffer: Arc<Mutex<Option<Vec<u8>>>>,
    // texture_handle: Arc<Mutex<Option<egui::TextureHandle>>>,
    texture_handle: egui::TextureHandle,
    frame_info: Arc<Mutex<Option<gst_video::VideoInfo>>>,
    // selected_stream_info: Arc<Mutex<Option<SelectedStream>>>,
) -> Result<PipelineData> {
    const TLS_DISABLE_VALIDATION: bool = true;
    // const TLS_DISABLE_VALIDATION: bool = false;

    // const CUSTOM_CA_CERT_PATH: Option<&str> = Some("ca_cert.pem");
    const CUSTOM_CA_CERT_PATH: Option<&str> = None;

    // let desired_width = 276;
    // let desired_height = 155;

    // let desired_width = 1710;
    // let desired_height = 960;

    // Create Elements
    let pipeline = gst::Pipeline::new();

    // Use rtspsrc instead of rtspsrc2
    let rtspsrc = gst::ElementFactory::make("rtspsrc") // Changed from rtspsrc2
        .name("source")
        .property("location", uri)
        .property("latency", 200u32) // milliseconds
        .property("protocols", gstreamer_rtsp::RTSPLowerTrans::TCP) // TCP often more reliable
        // Optional: You might need 'do-rtcp=true' for better sync/stats with some servers
        .property("do-rtcp", true)
        .build()
        .context("Failed to create rtspsrc element")?; // Updated context message

    // Configure TLS on rtspsrc
    if TLS_DISABLE_VALIDATION {
        debug!("WARNING: Disabling TLS certificate validation (tls-validation-flags=NONE). This is insecure!");
        // Use GIO flags to disable validation
        let flags = gio::TlsCertificateFlags::empty();
        rtspsrc.set_property("tls-validation-flags", &flags); // Pass flags by reference
    } else if let Some(ca_path_str) = CUSTOM_CA_CERT_PATH {
        debug!("Configuring custom CA certificate: {}", ca_path_str);
        let ca_path = std::path::Path::new(ca_path_str);

        // make path absolute
        let ca_path = std::fs::canonicalize(ca_path)
            .map_err(|e| anyhow!("Failed to canonicalize CA path '{}': {}", ca_path_str, e))?;

        if !ca_path.exists() {
            return Err(anyhow!(
                "Custom CA certificate file not found: {}",
                ca_path_str
            ));
        }
        debug!("Custom CA certificate file exists: {}", ca_path.display());

        // Create a TLS database and load the custom CA
        let tls_db = gio::TlsFileDatabase::new(ca_path).map_err(|e| {
            anyhow!(
                "Failed to create TLS database from CA file '{}': {}",
                ca_path_str,
                e
            )
        })?;

        // Set the custom database on rtspsrc
        rtspsrc.set_property("tls-database", &tls_db); // Pass db by reference
        debug!("Custom CA certificate loaded into TLS database for rtspsrc.");

        // let flags = gio::TlsCertificateFlags::VALIDATE_ALL;
        let flags = gio::TlsCertificateFlags::BAD_IDENTITY;
        rtspsrc.set_property("tls-validation-flags", &flags);
        // debug!("Explicitly set tls-validation-flags to VALIDATE_ALL.");
    } else {
        debug!("Using default system TLS certificate validation.");
        // No specific config needed; rtspsrc uses system CAs by default.
        // Ensure VALIDATE_ALL is the default (it usually is) or set it explicitly:
        // let flags = gio::TlsCertificateFlags::VALIDATE_ALL;
        let flags = gio::TlsCertificateFlags::BAD_IDENTITY;
        rtspsrc.set_property("tls-validation-flags", &flags);
    }

    let rtph264depay = gst::ElementFactory::make("rtph264depay")
        .name("depay")
        .build()
        .context("Failed to create rtph264depay element")?;

    let caps_filter = gst::ElementFactory::make("capsfilter")
        .name("resolution_filter")
        .build()
        .context("Failed to create capsfilter element")?;

    let resolution_caps = gst::Caps::builder("video/x-h264")
        .field("width", desired_res.0)
        .field("height", desired_res.1)
        .build();
    caps_filter.set_property("caps", &resolution_caps);

    let h264parse = gst::ElementFactory::make("h264parse")
        .name("parse")
        .build()
        .context("Failed to create h264parse element")?;

    let decoder = gst::ElementFactory::make("avdec_h264")
        .name("decode")
        .build()
        .context("Failed to create avdec_h264 element. Is gst-libav installed?")?;

    let videoconvert = gst::ElementFactory::make("videoconvert")
        .name("convert")
        .build()
        .context("Failed to create videoconvert element")?;

    // Configure AppSink
    let appsink = gst::ElementFactory::make("appsink")
        .name("sink")
        .build()
        .context("Failed to create appsink element")?
        .downcast::<gst_app::AppSink>()
        .map_err(|_| anyhow!("Element 'sink' is not an AppSink"))?;

    appsink.set_property("emit-signals", true);
    appsink.set_property("max-buffers", 5u32);
    appsink.set_property("drop", true);

    let caps_str = format!("video/x-raw,format={}", "RGBA");
    let sink_caps = gst::Caps::from_str(&caps_str)
        .map_err(|_| anyhow!("Failed to parse caps string: {}", caps_str))?;
    appsink.set_caps(Some(&sink_caps));

    // Add Elements to Pipeline
    pipeline
        .add_many(&[
            &rtspsrc,
            &rtph264depay,
            &caps_filter,
            &h264parse,
            &decoder,
            &videoconvert,
            appsink.upcast_ref(),
        ])
        .context("Failed to add elements to the pipeline")?;

    // Link Static Elements
    gst::Element::link_many(&[
        &rtph264depay,
        &caps_filter,
        &h264parse,
        &decoder,
        &videoconvert,
        appsink.upcast_ref(),
    ])
    .context("Failed to link static elements")?;

    // Connect Dynamic Pad for rtspsrc
    let rtph264depay_weak = rtph264depay.downgrade();
    rtspsrc.connect_pad_added(move |src, src_pad| {
        trace!(
            "Received new pad '{}' from '{}'",
            src_pad.name(),
            src.name()
        );

        // Check the pad's caps to ensure it's for H.264 video RTP stream
        let caps = match src_pad.current_caps() {
            Some(caps) => caps,
            None => {
                debug!("Pad '{}' has no caps yet, ignoring.", src_pad.name());
                return; // Can't determine type without caps
            }
        };
        let structure = match caps.structure(0) {
            Some(s) => s,
            None => {
                debug!("Pad '{}' caps has no structure, ignoring.", src_pad.name());
                return;
            }
        };

        // Check media type and encoding name more carefully
        let media_type = structure.get::<&str>("media").unwrap_or("");
        let encoding_name = structure.get::<&str>("encoding-name").unwrap_or("");

        debug!(
            "Pad '{}' details: media='{}', encoding='{}', caps='{}'",
            src_pad.name(),
            media_type,
            encoding_name,
            caps.to_string()
        );

        // We are looking for video encoded as H264
        if media_type == "video" && encoding_name.eq_ignore_ascii_case("H264") {
            trace!(
                "Pad '{}' is H.264 video. Attempting to link.",
                src_pad.name()
            );
        } else {
            trace!(
                "Pad '{}' is not the H.264 video stream we want ({}/{}), ignoring.",
                src_pad.name(),
                media_type,
                encoding_name
            );
            return;
        }

        // Get the sink pad of the depayloader
        let depay = match rtph264depay_weak.upgrade() {
            Some(depay) => depay,
            None => {
                warn!("Depayloader element already dropped!");
                return;
            }
        };
        let sink_pad = depay
            .static_pad("sink")
            .expect("rtph264depay should have a sink pad");

        // Check if the depayloader's sink pad is already linked
        if sink_pad.is_linked() {
            trace!(
                "Depayloader sink pad is already linked, ignoring '{}'",
                src_pad.name()
            );
            return;
        }

        // Attempt to link the rtspsrc pad to the depayloader sink pad
        match src_pad.link(&sink_pad) {
            Ok(_) => trace!(
                "Successfully linked '{}' to '{}'",
                src_pad.name(),
                sink_pad.name()
            ),
            Err(err) => warn!("Failed to link pads: {:?}", err),
        }
    });

    // Set up AppSink Callback
    let mut texture_handle_clone = texture_handle.clone();
    let frame_info_clone = frame_info.clone();
    appsink.set_callbacks(
        gst_app::AppSinkCallbacks::builder()
            .new_sample(move |sink| {
                let sample = match sink.pull_sample() {
                    Ok(sample) => sample,
                    // Err(gst::FlowError::Eos) => {
                    //     debug!("Appsink: Received EOS");
                    //     return Err(gst::FlowError::Eos);
                    // }
                    Err(err) => {
                        warn!("Appsink: Failed to pull sample: {:?}", err);
                        // return Err(err);
                        panic!();
                    }
                };

                let buffer = sample.buffer().ok_or_else(|| {
                    warn!("Appsink: Failed to get buffer from sample");
                    gst::FlowError::Error
                })?;
                let caps = sample.caps().ok_or_else(|| {
                    warn!("Appsink: Failed to get caps from sample");
                    gst::FlowError::Error
                })?;
                let info = gst_video::VideoInfo::from_caps(caps).map_err(|_| {
                    warn!("Appsink: Failed to get video info from caps");
                    gst::FlowError::Error
                })?;

                let map = buffer.map_readable().map_err(|_| {
                    warn!("Appsink: Failed to map buffer readable");
                    gst::FlowError::Error
                })?;

                let frame_data = map.as_slice();

                {
                    // let mut buffer = frame_buffer_clone.lock();
                    // let mut info_guard = frame_info_clone.lock();

                    // warn!("TODO: copy frame data to buffer");
                    // *buffer_guard = Some(frame_data.to_vec());

                    // debug!(
                    //     "Frame info updated: {}x{} Format: {:?}",
                    //     info.width(),
                    //     info.height(),
                    //     info.format()
                    // );
                    // /// write frame data to file
                    // let mut f = std::fs::File::create("test.dat").unwrap();
                    // std::io::Write::write_all(&mut f, frame_data).unwrap();
                    // panic!();

                    let img_size = [info.width() as _, info.height() as _];
                    let img = egui::ColorImage::from_rgba_unmultiplied(img_size, frame_data);

                    texture_handle_clone.set(img, Default::default());

                    #[cfg(feature = "nope")]
                    if let Some(buffer) = buffer.as_mut() {
                        let img_size = [info.width() as _, info.height() as _];
                        let img = egui::ColorImage::from_rgba_unmultiplied(img_size, frame_data);

                        buffer.set(img, Default::default());
                    }

                    // if info_guard.as_ref().map_or(true, |i| i != &info) {
                    //     *info_guard = Some(info);
                    // }
                }

                Ok(gst::FlowSuccess::Ok)
            })
            .build(),
    );

    Ok(PipelineData {
        pipeline,
        appsink,
        texture_handle,
        frame_info,
    })
}

pub fn run_gstreamer(
    id: PrinterId,
    desired_res: (u32, u32),
    start_time: &std::time::Instant,
    uri: &str,
    texture_handle: egui::TextureHandle,
    mut kill_rx: tokio::sync::mpsc::UnboundedReceiver<()>,
    mut cmd_rx: tokio::sync::mpsc::UnboundedReceiver<super::SubStreamCmd>,
    mut worker_tx: tokio::sync::mpsc::UnboundedSender<super::StreamWorkerMsg>,
    panic_cmd: StreamCmd,
) -> Result<()> {
    // TODO: Ensure GStreamer is initialized only once
    gst::init()?;

    let frame_info = Arc::new(Mutex::new(None::<gst_video::VideoInfo>));

    // let selected_stream_info = Arc::new(Mutex::new(None::<SelectedStream>));

    let pipeline_data = build_pipeline(
        desired_res,
        &uri,
        texture_handle.clone(),
        frame_info.clone(),
        // selected_stream_info.clone(),
    )
    .context("Failed to build pipeline")?;
    debug!("Pipeline built successfully.");

    let bus = pipeline_data
        .pipeline
        .bus()
        .context("Failed to get pipeline bus")?;

    let start_time2 = start_time.clone();

    // 5. Set up the bus watch to handle messages
    let pipeline_weak = pipeline_data.pipeline.downgrade(); // Use weak ref to avoid cycles
    let worker_tx = worker_tx.clone();

    let bus_handle = std::thread::spawn(move || {
        for msg in bus.iter_timed(gst::ClockTime::NONE) {
            if start_time2.elapsed() > std::time::Duration::from_secs(300) {
                debug!("Bus watcher: timeout seconds elapsed, exiting.");
                break;
            };

            let pipeline = match pipeline_weak.upgrade() {
                Some(p) => p,
                None => {
                    debug!("Bus watcher: Pipeline already dropped.");
                    break;
                }
            };

            match msg.view() {
                gst::MessageView::Error(err) => {
                    warn!(
                        "Error received from element {:?}: {}",
                        err.src().map(|s| s.path_string()),
                        err.error()
                    );
                    // warn!("Debugging information: {}", err.debug().unwrap_or("None"));
                    // Post an application message or trigger shutdown logic here
                    if let Err(e) = pipeline.set_state(gst::State::Null) {
                        warn!(
                            "Bus watcher: Failed to set pipeline to Null on error: {}",
                            e
                        );
                    }
                    break; // Exit loop on error
                }
                gst::MessageView::Eos(_) => {
                    debug!("Bus watcher: End-Of-Stream reached.");
                    // Trigger graceful shutdown if needed
                    if let Err(e) = pipeline.set_state(gst::State::Null) {
                        warn!("Bus watcher: Failed to set pipeline to Null on EOS: {}", e);
                    }
                    break; // Exit loop on EOS
                }
                gst::MessageView::StateChanged(state_changed) => {
                    // Optional: Log state changes for debugging
                    if state_changed.src().map(|s| s == &pipeline).unwrap_or(false) {
                        // debug!(
                        //     "Bus watcher: Pipeline state changed from {:?} to {:?} ({:?})",
                        //     state_changed.old(),
                        //     state_changed.current(),
                        //     state_changed.pending()
                        // );
                    }
                }
                _ => (),
            }
        }
        debug!("Bus watcher thread finished.");

        worker_tx
            .send(super::StreamWorkerMsg::Panic(id, panic_cmd))
            .unwrap();
        debug!("Bus watcher thread sent panic alert.");
    });

    // 6. Start the Pipeline
    pipeline_data
        .pipeline
        .set_state(gst::State::Playing)
        .context("Failed to set pipeline state to Playing")?;
    debug!("Pipeline state set to Playing. Waiting for stream...");

    // 7. Example: Periodically check the frame buffer (replace with your actual logic)
    // debug!("Starting frame reader loop (runs for 30 seconds).");
    // let start_time = std::time::Instant::now();

    let mut playing = true;

    loop {
        match kill_rx.try_recv() {
            Ok(_) => {
                debug!("Received kill signal, shutting down...");
                break;
            }
            Err(tokio::sync::mpsc::error::TryRecvError::Empty) => {
                // No message received, continue the loop
            }
            Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => {
                debug!("Kill channel closed, shutting down...");
                break;
            }
        }

        // doesn't work, leaves stream lagging behind
        #[cfg(feature = "nope")]
        match cmd_rx.try_recv() {
            Ok(super::SubStreamCmd::TogglePause) => {
                if playing {
                    playing = false;
                    pipeline_data
                        .pipeline
                        .set_state(gst::State::Paused)
                        .context("Failed to set pipeline state to Paused")?;
                } else {
                    playing = true;
                    pipeline_data
                        .pipeline
                        .set_state(gst::State::Playing)
                        .context("Failed to set pipeline state to Playing")?;
                }
            }
            _ => {}
        }
    }

    // 8. Shutdown
    debug!("Shutting down pipeline...");
    pipeline_data
        .pipeline
        .set_state(gst::State::Null)
        .context("Failed to set pipeline state to Null")?;
    debug!("Pipeline state set to Null.");

    // Wait for the bus watcher thread to finish
    if let Err(e) = bus_handle.join() {
        warn!("Error joining bus watcher thread: {:?}", e);
    }

    Ok(())
}

#[cfg(feature = "nope")]
pub fn test_gstreamer() -> Result<()> {
    // Initialize GStreamer
    gst::init()?;

    // Build the pipeline
    // let uri = "https://gstreamer.freedesktop.org/data/media/sintel_trailer-480p.webm";
    // let uri = "https://gstreamer.freedesktop.org/data/media/sintel_trailer-480p.webm";
    // let uri = "rtsp://localhost:8554/mystream";
    // let uri = "rtsp://camera:camera@192.168.0.147:554/stream1";

    let access_code = std::env::var("RTSP_PASS")?;
    let uri = format!(
        "rtsps://bblp:{}@192.168.0.23:322/streaming/live/1",
        access_code
    );

    let pipeline = gst::parse::launch(&format!("playbin uri={uri}"))?;

    debug!("Pipeline created");

    // Start playing
    let res = pipeline.set_state(gst::State::Playing)?;
    let is_live = res == gst::StateChangeSuccess::NoPreroll;

    let main_loop = glib::MainLoop::new(None, false);
    let main_loop_clone = main_loop.clone();
    let pipeline_weak = pipeline.downgrade();
    let bus = pipeline.bus().expect("Pipeline has no bus");
    let _bus_watch = bus
        .add_watch(move |_, msg| {
            let Some(pipeline) = pipeline_weak.upgrade() else {
                return glib::ControlFlow::Continue;
            };
            let main_loop = &main_loop_clone;
            match msg.view() {
                gst::MessageView::Error(err) => {
                    debug!(
                        "Error from {:?}: {} ({:?})",
                        err.src().map(|s| s.path_string()),
                        err.error(),
                        err.debug()
                    );
                    let _ = pipeline.set_state(gst::State::Ready);
                    main_loop.quit();
                }
                gst::MessageView::Eos(..) => {
                    // end-of-stream
                    let _ = pipeline.set_state(gst::State::Ready);
                    main_loop.quit();
                }
                gst::MessageView::Buffering(buffering) => {
                    // If the stream is live, we do not care about buffering
                    if is_live {
                        return glib::ControlFlow::Continue;
                    }

                    let percent = buffering.percent();
                    print!("Buffering ({percent}%)\r");
                    match std::io::stdout().flush() {
                        Ok(_) => {}
                        Err(err) => warn!("Failed: {err}"),
                    };

                    // Wait until buffering is complete before start/resume playing
                    if percent < 100 {
                        let _ = pipeline.set_state(gst::State::Paused);
                    } else {
                        let _ = pipeline.set_state(gst::State::Playing);
                    }
                }
                gst::MessageView::ClockLost(_) => {
                    // Get a new clock
                    let _ = pipeline.set_state(gst::State::Paused);
                    let _ = pipeline.set_state(gst::State::Playing);
                }
                _ => (),
            }
            glib::ControlFlow::Continue
        })
        .expect("Failed to add bus watch");

    main_loop.run();

    pipeline.set_state(gst::State::Null)?;

    Ok(())
}
