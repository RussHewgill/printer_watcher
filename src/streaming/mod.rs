use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

pub async fn write_frames(
    session: retina::client::Session<retina::client::Described>,
    stop_signal: std::pin::Pin<Box<dyn futures::Future<Output = Result<(), std::io::Error>>>>,
) -> Result<()> {
    use ffmpeg_next as ffmpeg;
    ffmpeg_next::init()?;

    debug!("write_frames");
    // let mut session = session.play(retina::client::PlayOptions::default()).await?;

    let mut session = session
        .play(retina::client::PlayOptions::default())
        .await?
        .demuxed()?;

    tokio::pin!(stop_signal);

    #[cfg(feature = "nope")]
    {
        let codec = ffmpeg::decoder::find(ffmpeg::codec::Id::H264).unwrap();

        debug!("codec: {}", codec.name());

        let context_decoder = ffmpeg::codec::context::Context::new_with_codec(codec);

        // debug!("context_decoder: {:#?}", context_decoder);

        debug!(
            "context codec: {:?}",
            context_decoder.codec().unwrap().name()
        );
        debug!("frame rate: {:?}", context_decoder.frame_rate());
        debug!("medium: {:?}", context_decoder.medium());

        debug!("getting decoder");
        let mut decoder = context_decoder.decoder().video().unwrap();
        debug!("got decoder");

        let mut scaler = ffmpeg::software::scaling::context::Context::get(
            decoder.format(),
            decoder.width(),
            decoder.height(),
            ffmpeg_next::format::Pixel::RGB24,
            decoder.width(),
            decoder.height(),
            ffmpeg_next::software::scaling::Flags::BILINEAR,
        )?;

        let mut frame_index = 0;

        debug!("getting first packet");
        match futures::StreamExt::next(&mut session).await {
            Some(Ok(retina::client::PacketItem::Rtp(pkt))) => {
                debug!("got RTP packet");
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

    // #[cfg(feature = "nope")]
    loop {
        debug!("waiting for frame");
        let f = futures::StreamExt::next(&mut session)
            .await
            .unwrap()
            .unwrap();

        debug!("got frame");

        // #[cfg(feature = "nope")]
        match f {
            retina::codec::CodecItem::VideoFrame(f) => {
                let stream = &session.streams()[f.stream_id()];
                let start_ctx = *f.start_ctx();

                // debug!("loss: {:?}", f.loss());
                debug!("is RAP: {:?}", f.is_random_access_point());
                // debug!("timestampe: {:?}", f.timestamp());

                let mut data = f.into_data();

                debug!("frame len: {:?}", data.len());

                {
                    let params = stream.parameters().unwrap();
                    // debug!("params: {:#?}", params);

                    let codec = ffmpeg::decoder::find(ffmpeg::codec::Id::H264).unwrap();

                    debug!("codec: {}", codec.name());

                    let context_decoder = ffmpeg::codec::context::Context::new_with_codec(codec);
                    debug!("getting decoder");
                    // ffmpeg::codec::context::Context::from_parameters(&params)?;
                    let mut decoder = context_decoder.decoder().video()?;
                    debug!("got decoder");

                    debug!("decoder format: {:?}", decoder.format());

                    let mut scaler = ffmpeg::software::scaling::context::Context::get(
                        decoder.format(),
                        decoder.width(),
                        decoder.height(),
                        ffmpeg_next::format::Pixel::RGB24,
                        decoder.width(),
                        decoder.height(),
                        ffmpeg_next::software::scaling::Flags::BILINEAR,
                    )?;
                    debug!("got scaler");

                    let mut frame_index = 0;

                    // let mut receive_and_process_decoded_frames =
                    //     |decoder: &mut ffmpeg::decoder::Video| -> Result<(), ffmpeg::Error> {
                    //         let mut decoded = ffmpeg::util::frame::video::Video::empty();
                    //         while decoder.receive_frame(&mut decoded).is_ok() {
                    //             let mut rgb_frame = ffmpeg::util::frame::video::Video::empty();
                    //             scaler.run(&decoded, &mut rgb_frame)?;
                    //             save_file(&rgb_frame, frame_index).unwrap();
                    //             frame_index += 1;
                    //         }
                    //         Ok(())
                    //     };

                    use ffmpeg::util::frame::video::Video;

                    let mut decoded = Video::empty();

                    decoder.receive_frame(&mut decoded)?;

                    let mut rgb_frame = Video::empty();

                    scaler.run(&decoded, &mut rgb_frame)?;
                    save_file(&rgb_frame, frame_index).unwrap();
                    // frame_index += 1;

                    // let mut rgb_frame = ffmpeg::util::frame::video::Video::empty();
                    // scaler.run(&decoded, &mut rgb_frame)?;
                    // save_file(&rgb_frame, frame_index).unwrap();
                    // frame_index += 1;

                    break;
                    //
                }

                #[cfg(feature = "nope")]
                {
                    use ffmpeg_next as ffmpeg;

                    // // Add Annex B start code if it's not present
                    // if !data.starts_with(&[0, 0, 0, 1]) && !data.starts_with(&[0, 0, 1]) {
                    //     data = [0, 0, 0, 1]
                    //         .iter()
                    //         .cloned()
                    //         .chain(data.iter().cloned())
                    //         .collect();
                    // }

                    let mut packet = ffmpeg::Packet::copy(&data);

                    let decoder = ffmpeg::codec::Context::new();
                    let mut decoder_config = decoder.decoder().open_as(ffmpeg::codec::Id::H264)?;

                    /// Decode the packet into a frame
                    let mut decoded_frame = ffmpeg::frame::Video::empty();
                    decoder_config.send_packet(&packet)?;
                    decoder_config.receive_frame(&mut decoded_frame)?;

                    /// Convert the decoded frame to RGB
                    let mut rgb_frame = ffmpeg::frame::Video::empty();
                    let mut scaler = ffmpeg::software::scaling::Context::get(
                        decoded_frame.format(),
                        decoded_frame.width(),
                        decoded_frame.height(),
                        ffmpeg::util::format::Pixel::RGB24,
                        decoded_frame.width(),
                        decoded_frame.height(),
                        ffmpeg::software::scaling::Flags::BILINEAR,
                    )?;
                    scaler.run(&decoded_frame, &mut rgb_frame)?;

                    // Create an ImageBuffer from the RGB frame
                    let width = rgb_frame.width() as u32;
                    let height = rgb_frame.height() as u32;
                    let buffer = rgb_frame.data(0);
                    let img =
                        image::ImageBuffer::<image::Rgb<u8>, _>::from_raw(width, height, buffer)
                            .unwrap();

                    // Save as JPEG
                    img.save("output.jpg")?;

                    break;

                    //
                }

                //
            }
            retina::codec::CodecItem::MessageFrame(msg) => {
                info!("message: {:?}", msg);
            }
            _ => warn!("unexpected item"),
        }
    }

    #[cfg(feature = "nope")]
    loop {
        tokio::select! {
            pkt = futures::StreamExt::next(&mut session) => {
                match pkt.ok_or_else(|| anyhow!("EOF"))?? {
                    retina::codec::CodecItem::VideoFrame(f) => {
                        debug!("got frame");

                        let stream = &session.streams()[f.stream_id()];
                        let start_ctx = *f.start_ctx();

                        let data = f.data();

                        //
                    },
                    _ => continue,
                };
            },
            _ = &mut stop_signal => {
                info!("Stopping due to signal");
                break;
            },
        }
        break;
    }

    Ok(())
}

fn save_file(
    frame: &ffmpeg_next::util::frame::video::Video,
    index: usize,
) -> std::result::Result<(), std::io::Error> {
    let mut file = std::fs::File::create(format!("frame{}.ppm", index))?;
    std::io::Write::write_all(
        &mut file,
        format!("P6\n{} {}\n255\n", frame.width(), frame.height()).as_bytes(),
    )?;
    std::io::Write::write_all(&mut file, frame.data(0))?;
    Ok(())
}

/// Writes `.jpeg` files to the specified directory.
async fn write_jpeg(
    session: retina::client::Session<retina::client::Described>,
    stop_signal: std::pin::Pin<Box<dyn futures::Future<Output = Result<(), std::io::Error>>>>,
) -> Result<()> {
    let mut session = session
        .play(
            retina::client::PlayOptions::default()
                // .initial_timestamp(opts.initial_timestamp)
                .enforce_timestamps_with_max_jump_secs(std::num::NonZeroU32::new(10).unwrap()),
        )
        .await?
        .demuxed()?;

    let duration = None;

    let out_dir = std::path::PathBuf::from(".");

    let sleep = match duration {
        Some(secs) => {
            futures::future::Either::Left(tokio::time::sleep(std::time::Duration::from_secs(secs)))
        }
        None => futures::future::Either::Right(futures::future::pending()),
    };
    tokio::pin!(stop_signal);
    tokio::pin!(sleep);

    let mut frame_count = 0;

    loop {
        tokio::select! {
            pkt = futures::StreamExt::next(&mut session) => {
                match pkt.ok_or_else(|| anyhow!("EOF"))?? {
                    retina::codec::CodecItem::VideoFrame(f) => {
                        let out_path = out_dir.join(&format!("{frame_count:05}.jpeg"));
                        std::fs::write(out_path, f.data())?;

                        frame_count += 1;
                    },
                    retina::codec::CodecItem::Rtcp(rtcp) => {
                        if let (Some(t), Some(Ok(Some(sr)))) = (rtcp.rtp_timestamp(), rtcp.pkts().next().map(retina::rtcp::PacketRef::as_sender_report)) {
                            println!("{}: SR ts={}", t, sr.ntp_timestamp());
                        }
                    },
                    _ => continue,
                };
            },
            _ = &mut stop_signal => {
                info!("Stopping due to signal");
                break;
            },
            _ = &mut sleep => {
                info!("Stopping after {} seconds", duration.unwrap());
                break;
            },
        }
    }

    Ok(())
}
