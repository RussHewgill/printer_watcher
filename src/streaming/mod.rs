pub mod rtsp;

use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

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
