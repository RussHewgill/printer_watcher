use std::sync::Arc;

use atomic::Atomic;
use egui::ColorImage;

pub type ApplyVideoFrameFn = Box<dyn FnMut(ColorImage) + Send>;

#[derive(Clone, Debug)]
/// Simple concurrecy of primitive values.
pub struct Shared<T: Copy> {
    raw_value: Arc<Atomic<T>>,
}

impl<T: Copy> Shared<T> {
    /// Set the value.
    pub fn set(&self, value: T) {
        self.raw_value.store(value, atomic::Ordering::Relaxed)
    }
    /// Get the value.
    pub fn get(&self) -> T {
        self.raw_value.load(atomic::Ordering::Relaxed)
    }
    /// Make a new cache.
    pub fn new(value: T) -> Self {
        Self {
            raw_value: Arc::new(Atomic::new(value)),
        }
    }
}

#[derive(PartialEq, Clone, Copy, Debug)]
/// The possible states of a [`Player`].
pub enum PlayerState {
    /// No playback.
    Stopped,
    /// Streams have reached the end of the file.
    EndOfFile,
    /// Stream is seeking. Inner bool represents whether or not the seek is currently in progress.
    Seeking(bool),
    /// Playback is paused.
    Paused,
    /// Playback is ongoing.
    Playing,
    /// Playback is scheduled to restart.
    Restarting,
}
