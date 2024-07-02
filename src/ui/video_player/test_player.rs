use egui::Vec2;

use super::VideoPlayer;

pub struct TestVideoApp {
    pub video_player: VideoPlayer,
    pub stream_tx: tokio::sync::mpsc::UnboundedSender<crate::streaming::StreamCmd>,
}

/// new
impl TestVideoApp {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        stream_tx: tokio::sync::mpsc::UnboundedSender<crate::streaming::StreamCmd>,
        // cmd_tx: tokio::sync::mpsc::UnboundedSender<PrinterConnCmd>,
        // msg_rx: tokio::sync::mpsc::UnboundedReceiver<PrinterConnMsg>,
    ) -> Self {
        let video_player = VideoPlayer::new("test_player", &cc.egui_ctx, Vec2::new(640., 480.));

        let creds = crate::streaming::rtsp::RtspCreds {
            host: std::env::var("RTSP_URL").unwrap(),
            username: std::env::var("RTSP_USER").unwrap(),
            password: std::env::var("RTSP_PASS").unwrap(),
        };

        let id = std::env::var("PRUSA_ID").unwrap();
        let id: crate::config::printer_id::PrinterId = id.into();
        stream_tx
            .send(crate::streaming::StreamCmd::StartRtsp(
                id,
                video_player.texture_handle.clone(),
                creds,
            ))
            .unwrap();

        Self {
            video_player,
            stream_tx,
        }
    }
}

impl eframe::App for TestVideoApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.label("Test App");

            self.video_player.ui(ui, Vec2::new(640., 480.));

            //
        });
    }
}
