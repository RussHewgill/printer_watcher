pub mod test_player;

use egui::{load::SizedTexture, Rect, Response, Sense, TextureHandle, Ui, Vec2};

pub struct VideoPlayer {
    /// The player's texture handle.
    pub texture_handle: TextureHandle,
    /// The size of the video stream.
    pub size: Vec2,
    ctx_ref: egui::Context,
}

impl VideoPlayer {
    pub fn new(id: &str, ctx: &egui::Context, size: Vec2) -> Self {
        let image = egui::ColorImage::new(
            [size.x as usize, size.y as usize],
            egui::Color32::from_gray(0),
        );
        let texture_handle =
            ctx.load_texture(format!("{}_texture", &id), image, Default::default());
        Self {
            texture_handle,
            size,
            ctx_ref: ctx.clone(),
        }
    }

    /// Process player state updates. This function must be called for proper function
    /// of the player. This function is already included in  [`Player::ui`] or
    /// [`Player::ui_at`].
    pub fn process_state(&mut self) {
        //
    }

    /// Create the [`egui::Image`] for the video frame.
    pub fn generate_frame_image(&self, size: Vec2) -> egui::Image {
        egui::Image::new(SizedTexture::new(self.texture_handle.id(), size)).sense(Sense::click())
    }

    /// Draw the video frame with a specific rect (without controls). Make sure to call [`Player::process_state`].
    pub fn render_frame(&self, ui: &mut Ui, size: Vec2) -> Response {
        ui.add(self.generate_frame_image(size))
    }

    /// Draw the video frame (without controls). Make sure to call [`Player::process_state`].
    pub fn render_frame_at(&self, ui: &mut Ui, rect: Rect) -> Response {
        ui.put(rect, self.generate_frame_image(rect.size()))
    }

    /// Draw the video frame and player controls and process state changes.
    pub fn ui(&mut self, ui: &mut Ui, size: Vec2) -> egui::Response {
        let frame_response = self.render_frame(ui, size);
        // self.render_controls(ui, &frame_response);
        // self.render_subtitles(ui, &frame_response);
        self.process_state();
        frame_response
    }

    /// Draw the video frame and player controls with a specific rect, and process state changes.
    pub fn ui_at(&mut self, ui: &mut Ui, rect: Rect) -> egui::Response {
        let frame_response = self.render_frame_at(ui, rect);
        // self.render_controls(ui, &frame_response);
        // self.render_subtitles(ui, &frame_response);
        self.process_state();
        frame_response
    }
}