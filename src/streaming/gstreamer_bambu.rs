use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use parking_lot::Mutex;
use std::{cell::LazyCell, io::Write, str::FromStr, sync::Arc};

use gst::prelude::*;
use gstreamer::{self as gst, glib::FlagsClass};
use gstreamer_app as gst_app;
use gstreamer_video as gst_video;

pub struct GStreamerPlayer {
    pub uri: String,
    // texture_handle: Option<egui::TextureHandle>,
    texture_handle: Arc<Mutex<Option<egui::TextureHandle>>>,
    cmd_rx: crossbeam_channel::Receiver<crate::streaming::StreamCmd>,
}

impl GStreamerPlayer {
    pub fn new(
        username: &str,
        password: &str,
        host: &str,
        port: u16,
        // port: u16,
        cmd_rx: crossbeam_channel::Receiver<crate::streaming::StreamCmd>,
    ) -> Self {
        Self {
            uri: format!("rtsp://{username}:{password}@{host}:{port}/stream1"),
            texture_handle: Arc::new(Mutex::new(None)),
            cmd_rx,
        }
    }

    pub fn init(&self) -> Result<()> {
        // let uri = "rtsp://camera:camera@192.168.0.147:554/stream1";

        let access_code = std::env::var("RTSP_PASS")?;
        let uri = format!(
            "rtsps://bblp:{}@192.168.0.23:322/streaming/live/1",
            access_code
        );

        let cmd_rx2 = self.cmd_rx.clone();
        let texture_handle2 = self.texture_handle.clone();

        std::thread::spawn(move || loop {
            match cmd_rx2.recv() {
                Ok(crate::streaming::StreamCmd::StartBambuStills {
                    id,
                    host,
                    access_code,
                    serial,
                    texture,
                }) => {
                    debug!("Received StartBambuStills command");

                    texture_handle2.lock().replace(texture);
                    break;
                }
                _ => {
                    debug!("Received unknown command");
                }
            }
        });

        test_gstreamer(&uri, self.texture_handle.clone())?;

        Ok(())
    }
}

struct PipelineData {
    pipeline: gst::Pipeline,
    appsink: gst_app::AppSink,
    // frame_buffer: Arc<Mutex<Option<Vec<u8>>>>,
    texture_handle: Arc<Mutex<Option<egui::TextureHandle>>>,
    // Keep track of width/height/format once known
    frame_info: Arc<Mutex<Option<gst_video::VideoInfo>>>,
}

fn build_pipeline(
    uri: &str,
    // frame_buffer: Arc<Mutex<Option<Vec<u8>>>>,
    texture_handle: Arc<Mutex<Option<egui::TextureHandle>>>,
    frame_info: Arc<Mutex<Option<gst_video::VideoInfo>>>,
) -> Result<PipelineData> {
    // --- Create Elements ---
    let pipeline = gst::Pipeline::new();

    // *** Use rtspsrc instead of rtspsrc2 ***
    let rtspsrc = gst::ElementFactory::make("rtspsrc") // Changed from rtspsrc2
        .name("source")
        .property("location", uri)
        .property("latency", 200u32) // milliseconds
        .property("protocols", gstreamer_rtsp::RTSPLowerTrans::TCP) // TCP often more reliable
        // .property("protocols", gstreamer_rtsp::RTSPLowerTrans::TLS) // TCP often more reliable
        // Optional: You might need 'do-rtcp=true' for better sync/stats with some servers
        // .property("do-rtcp", true)
        .build()
        .context("Failed to create rtspsrc element")?; // Updated context message

    const TLS_DISABLE_VALIDATION: bool = false;
    const CUSTOM_CA_CERT_PATH: Option<&str> = Some("./ca_cert.pem"); // e.g., Some("/path/to/your/ca.pem");

    // --- Configure TLS on rtspsrc ---
    if TLS_DISABLE_VALIDATION {
        debug!("WARNING: Disabling TLS certificate validation (tls-validation-flags=NONE). This is insecure!");
        // Use GIO flags to disable validation
        let flags = gio::TlsCertificateFlags::empty();
        rtspsrc.set_property("tls-validation-flags", &flags); // Pass flags by reference
    } else if let Some(ca_path_str) = CUSTOM_CA_CERT_PATH {
        debug!("Configuring custom CA certificate: {}", ca_path_str);
        let ca_path = std::path::Path::new(ca_path_str);
        if !ca_path.exists() {
            return Err(anyhow!(
                "Custom CA certificate file not found: {}",
                ca_path_str
            ));
        }

        // Create a TLS database and load the custom CA
        let tls_db = gio::TlsFileDatabase::new(ca_path).map_err(|e| {
            anyhow!(
                "Failed to create TLS database from CA file '{}': {}",
                ca_path_str,
                e
            )
        })?;

        // Set the custom database on rtspsrc
        // The property expects a GTlsDatabase object. We pass our TlsFileDatabase.
        rtspsrc.set_property("tls-database", &tls_db); // Pass db by reference
        debug!("Custom CA certificate loaded into TLS database for rtspsrc.");
        // Default validation flags will be used (VALIDATE_ALL), but now against the custom DB + system CAs
        // You could explicitly set VALIDATE_ALL if needed:
        // let flags = gio::TlsCertificateFlags::VALIDATE_ALL;
        // rtspsrc.set_property("tls-validation-flags", &flags);
    } else {
        debug!("Using default system TLS certificate validation.");
        // No specific config needed; rtspsrc uses system CAs by default.
        // Ensure VALIDATE_ALL is the default (it usually is) or set it explicitly:
        let flags = gio::TlsCertificateFlags::VALIDATE_ALL;
        rtspsrc.set_property("tls-validation-flags", &flags);
    }

    let rtph264depay = gst::ElementFactory::make("rtph264depay")
        .name("depay")
        .build()
        .context("Failed to create rtph264depay element")?;

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

    // --- Configure AppSink (same as before) ---
    let appsink = gst::ElementFactory::make("appsink")
        .name("sink")
        .build()
        .context("Failed to create appsink element")?
        .downcast::<gst_app::AppSink>()
        .map_err(|_| anyhow!("Element 'sink' is not an AppSink"))?;

    appsink.set_property("emit-signals", true);
    appsink.set_property("max-buffers", 5u32);
    appsink.set_property("drop", true);

    // let caps_str = format!("video/x-raw,format={}", "BGR");
    let caps_str = format!("video/x-raw,format={}", "RGBA");
    let sink_caps = gst::Caps::from_str(&caps_str)
        .map_err(|_| anyhow!("Failed to parse caps string: {}", caps_str))?;
    appsink.set_caps(Some(&sink_caps));

    // --- Add Elements to Pipeline ---
    pipeline
        .add_many(&[
            &rtspsrc,
            &rtph264depay,
            &h264parse,
            &decoder,
            &videoconvert,
            appsink.upcast_ref(),
        ])
        .context("Failed to add elements to the pipeline")?;

    // --- Link Static Elements (same as before) ---
    gst::Element::link_many(&[
        &rtph264depay,
        &h264parse,
        &decoder,
        &videoconvert,
        appsink.upcast_ref(),
    ])
    .context("Failed to link static elements")?;

    // --- Connect Dynamic Pad for rtspsrc ---
    // The logic here is identical to the rtspsrc2 version, but it's even more
    // important with rtspsrc as it might emit pads for audio/metadata too.
    // We must ensure we only link the *video* pad.
    let rtph264depay_weak = rtph264depay.downgrade();
    rtspsrc.connect_pad_added(move |src, src_pad| {
        debug!(
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
            debug!(
                "Pad '{}' is H.264 video. Attempting to link.",
                src_pad.name()
            );
        } else {
            debug!(
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
            debug!(
                "Depayloader sink pad is already linked, ignoring '{}'",
                src_pad.name()
            );
            return;
        }

        // Attempt to link the rtspsrc pad to the depayloader sink pad
        match src_pad.link(&sink_pad) {
            Ok(_) => debug!(
                "Successfully linked '{}' to '{}'",
                src_pad.name(),
                sink_pad.name()
            ),
            Err(err) => warn!("Failed to link pads: {:?}", err),
        }
    });

    // --- Set up AppSink Callback (same as before) ---
    let frame_buffer_clone = texture_handle.clone();
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
                    let mut buffer = frame_buffer_clone.lock();
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

                    if let Some(buffer) = buffer.as_mut() {
                        let img_size = [info.width() as _, info.height() as _];
                        let img = egui::ColorImage::from_rgba_unmultiplied(img_size, frame_data);

                        buffer.set(img, Default::default());
                    }

                    #[cfg(feature = "nope")]
                    if let Some(buffer) = buffer.as_mut() {
                        match image::load_from_memory(&frame_data) {
                            Ok(image) => {
                                let img_size = [image.width() as _, image.height() as _];
                                let image_buffer = image.to_rgba8();
                                let pixels = image_buffer.as_flat_samples();
                                let img = egui::ColorImage::from_rgba_unmultiplied(
                                    img_size,
                                    pixels.as_slice(),
                                );

                                buffer.set(img, Default::default());
                            }
                            Err(e) => {
                                error!("Failed to load image from memory: {}", e);
                                // return Err(gst::FlowError::Error);
                            }
                        }
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

pub fn test_gstreamer(
    uri: &str,
    texture_handle: Arc<Mutex<Option<egui::TextureHandle>>>,
    //
) -> Result<()> {
    // TODO: Ensure GStreamer is initialized only once
    gst::init()?;

    // let frame_buffer = Arc::new(Mutex::new(None::<Vec<u8>>));
    let frame_info = Arc::new(Mutex::new(None::<gst_video::VideoInfo>));

    let pipeline_data = build_pipeline(&uri, texture_handle.clone(), frame_info.clone())
        .context("Failed to build pipeline")?;
    debug!("Pipeline built successfully.");

    let bus = pipeline_data
        .pipeline
        .bus()
        .context("Failed to get pipeline bus")?;

    // 5. Set up the bus watch to handle messages
    let pipeline_weak = pipeline_data.pipeline.downgrade(); // Use weak ref to avoid cycles
    let bus_handle = std::thread::spawn(move || {
        for msg in bus.iter_timed(gst::ClockTime::NONE) {
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
    });

    // 6. Start the Pipeline
    pipeline_data
        .pipeline
        .set_state(gst::State::Playing)
        .context("Failed to set pipeline state to Playing")?;
    debug!("Pipeline state set to Playing. Waiting for stream...");

    // 7. Example: Periodically check the frame buffer (replace with your actual logic)
    // debug!("Starting frame reader loop (runs for 30 seconds).");
    let start_time = std::time::Instant::now();
    while start_time.elapsed() < std::time::Duration::from_secs(30) {
        std::thread::sleep(std::time::Duration::from_millis(500)); // Check every 500ms

        // match frame_info.lock().as_ref() {
        //     Some(info) => {
        //         debug!(
        //             "[{:.1?}] Latest Frame Info: {}x{} Format: {:?}",
        //             start_time.elapsed(),
        //             info.width(),
        //             info.height(),
        //             info.format()
        //         );
        //     }
        //     None => {
        //         debug!("No frame info available yet.");
        //     }
        // }

        // Add a check to see if the pipeline is still running
        if pipeline_data.pipeline.current_state() != gst::State::Playing {
            debug!("Frame reader loop: Pipeline is no longer playing. Exiting loop.");
            break;
        }
    }
    debug!("Frame reader loop finished.");

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
