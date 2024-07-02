use ffmpeg_next as ffmpeg;

use types::{ApplyVideoFrameFn, PlayerState, Shared};

pub mod types;

/// The [`Player`] processes and controls streams of video/audio. This is what you use to show a video file.
/// Initialize once, and use the [`Player::ui`] or [`Player::ui_at()`] functions to show the playback.
pub struct Player {
    /// The video streamer of the player.
    pub video_streamer: Arc<Mutex<VideoStreamer>>,
    /// The state of the player.
    pub player_state: Shared<PlayerState>,
    /// The player's texture handle.
    pub texture_handle: TextureHandle,
}

/// Streams video.
pub struct VideoStreamer {
    video_decoder: ffmpeg::decoder::Video,
    video_stream_index: usize,
    player_state: Shared<PlayerState>,
    duration_ms: i64,
    input_context: ffmpeg::format::context::input::Input,
    video_elapsed_ms: Shared<i64>,
    _audio_elapsed_ms: Shared<i64>,
    apply_video_frame_fn: Option<ApplyVideoFrameFn>,
}

impl Player {
    fn reset(&mut self) {
        self.video_streamer.lock().reset();
    }

    /// Pause the stream.
    pub fn pause(&mut self) {
        self.set_state(PlayerState::Paused)
    }
    /// Resume the stream from a paused state.
    pub fn resume(&mut self) {
        self.set_state(PlayerState::Playing)
    }

    /// Process player state updates. This function must be called for proper function
    /// of the player. This function is already included in  [`Player::ui`] or
    /// [`Player::ui_at`].
    pub fn process_state(&mut self) {
        let mut reset_stream = false;

        match self.player_state.get() {
            PlayerState::EndOfFile => {
                if self.options.looping {
                    reset_stream = true;
                } else {
                    self.player_state.set(PlayerState::Stopped);
                }
            }
            PlayerState::Stopped => {
                self.stop_direct();
            }
            PlayerState::Playing => {
                for subtitle in self.current_subtitles.iter_mut() {
                    subtitle.remaining_duration_ms -=
                        self.ctx_ref.input(|i| (i.stable_dt * 1000.) as i64);
                }
                self.current_subtitles
                    .retain(|s| s.remaining_duration_ms > 0);
                if let Some(mut queue) = self.subtitles_queue.try_lock() {
                    if queue.len() > 1 {
                        self.current_subtitles.push(queue.pop_front().unwrap());
                    }
                }
            }
            PlayerState::Seeking(seek_in_progress) => {
                if self.last_seek_ms.is_some() {
                    let last_seek_ms = *self.last_seek_ms.as_ref().unwrap();
                    if !seek_in_progress {
                        if let Some(previeous_player_state) = self.preseek_player_state {
                            self.set_state(previeous_player_state)
                        }
                        self.video_elapsed_ms_override = None;
                        self.last_seek_ms = None;
                    } else {
                        self.video_elapsed_ms_override = Some(last_seek_ms);
                    }
                } else {
                    self.video_elapsed_ms_override = None;
                }
            }
            PlayerState::Restarting => reset_stream = true,
            _ => (),
        }
        if let Ok(message) = self.message_reciever.try_recv() {
            fn increment_stream_info(stream_info: &mut (usize, usize)) {
                stream_info.0 = ((stream_info.0 + 1) % (stream_info.1 + 1)).max(1);
            }
            match message {
                PlayerMessage::StreamCycled(stream_type) => match stream_type {
                    ffmpeg::media::Type::Audio => {
                        // increment_stream_info(&mut self.audio_stream_info)
                        unimplemented!()
                    }
                    ffmpeg::media::Type::Subtitle => {
                        // self.current_subtitles.clear();
                        // increment_stream_info(&mut self.subtitle_stream_info);
                        unimplemented!()
                    }
                    _ => unreachable!(),
                },
            }
        }
        if reset_stream {
            self.reset();
            self.resume();
        }
    }
}
