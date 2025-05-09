use egui::Vec2;

use super::VideoPlayer;

pub struct TestVideoApp {
    pub video_player: VideoPlayer,
    // pub stream_tx: tokio::sync::mpsc::UnboundedSender<crate::streaming::StreamCmd>,
    pub stream_tx: crossbeam_channel::Sender<crate::streaming::StreamCmd>,
}

#[cfg(feature = "nope")]
/// new
impl TestVideoApp {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        stream_tx: crossbeam_channel::Sender<crate::streaming::StreamCmd>,
        // stream_tx: tokio::sync::mpsc::UnboundedSender<crate::streaming::StreamCmd>,
        // cmd_tx: tokio::sync::mpsc::UnboundedSender<PrinterConnCmd>,
        // msg_rx: tokio::sync::mpsc::UnboundedReceiver<PrinterConnMsg>,
    ) -> Self {
        /// 276 x 155.25
        let thumbnail_width = crate::ui::PRINTER_WIDGET_SIZE.0 - 24.;
        let thumbnail_height = thumbnail_width * 0.5625;

        let video_player = VideoPlayer::new(
            "test_player",
            &cc.egui_ctx,
            Vec2::new(thumbnail_width, thumbnail_height),
            stream_tx.clone(),
        );

        // let creds = crate::streaming::rtsp::RtspCreds {
        //     host: std::env::var("RTSP_URL").unwrap(),
        //     username: std::env::var("RTSP_USER").unwrap(),
        //     password: std::env::var("RTSP_PASS").unwrap(),
        // };

        // let id = std::env::var("PRUSA_ID").unwrap();
        // let id: crate::config::printer_id::PrinterId = id.into();
        // stream_tx
        //     .send(crate::streaming::StreamCmd::StartRtsp(
        //         id,
        //         video_player.texture_handle.clone(),
        //         creds,
        //         cc.egui_ctx.clone(),
        //     ))
        //     .unwrap();

        Self {
            video_player,
            stream_tx,
        }
    }
}

#[cfg(feature = "nope")]
impl eframe::App for TestVideoApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.label("Test App");

            // self.video_player.ui(ui, Vec2::new(640., 480.));
            self.video_player.ui(ui);

            //
        });
    }
}
