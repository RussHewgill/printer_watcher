use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};
use tracing_subscriber::field::debug;

#[derive(Default, Debug)]
pub struct Sps {
    pub profile_idc: u8, // u(8)
    flag: u8,

    pub level_idc: u8,         // u(8)
    seq_parameter_set_id: u32, // ue(v)

    chroma_format_idc: u32, // ue(v)

    separate_colour_plane_flag: u8,           // u(1)
    bit_depth_luma_minus8: u32,               // ue(v)
    bit_depth_chroma_minus8: u32,             // ue(v)
    qpprime_y_zero_transform_bypass_flag: u8, // u(1)

    seq_scaling_matrix_present_flag: u8, // u(1)

    seq_scaling_list_present_flag: Vec<u8>, // u(1)

    log2_max_frame_num_minus4: u32, // ue(v)
    pic_order_cnt_type: u32,        // ue(v)

    log2_max_pic_order_cnt_lsb_minus4: u32, // ue(v)

    delta_pic_order_always_zero_flag: u8,       // u(1)
    offset_for_non_ref_pic: i32,                // se(v)
    offset_for_top_to_bottom_field: i32,        // se(v)
    num_ref_frames_in_pic_order_cnt_cycle: u32, // ue(v)

    offset_for_ref_frame: Vec<i32>, // se(v)

    max_num_ref_frames: u32,                  // ue(v)
    gaps_in_frame_num_value_allowed_flag: u8, // u(1)

    pic_width_in_mbs_minus1: u32,        // ue(v)
    pic_height_in_map_units_minus1: u32, // ue(v)
    frame_mbs_only_flag: u8,             // u(1)

    mb_adaptive_frame_field_flag: u8, // u(1)

    direct_8x8_inference_flag: u8, // u(1)

    frame_cropping_flag: u8, // u(1)

    frame_crop_left_offset: u32,   // ue(v)
    frame_crop_right_offset: u32,  // ue(v)
    frame_crop_top_offset: u32,    // ue(v)
    frame_crop_bottom_offset: u32, // ue(v)

    vui_parameters_present_flag: u8, // u(1)
}

impl Sps {
    pub fn parse_from(data: &[u8]) -> Result<Self> {
        let mut out = Sps::default();

        // let mut r = std::io::Cursor::new(extra);
        let mut r = bitreader::BitReader::new(&data);

        out.profile_idc = r.read_u8(8)?;
        out.flag = r.read_u8(8)?;
        out.level_idc = r.read_u8(8)?;
        out.seq_parameter_set_id = read_uev(&mut r)?;

        match out.profile_idc {
            100 | 110 | 122 | 244 | 44 | 83 | 86 | 118 | 128 => {
                out.chroma_format_idc = read_uev(&mut r)?;
                if out.chroma_format_idc == 3 {
                    out.separate_colour_plane_flag = r.read_u8(1)?;
                }
                out.bit_depth_luma_minus8 = read_uev(&mut r)?;
                out.bit_depth_chroma_minus8 = read_uev(&mut r)?;

                out.qpprime_y_zero_transform_bypass_flag = r.read_u8(1)?;
                out.seq_scaling_matrix_present_flag = r.read_u8(1)?;

                if out.seq_scaling_matrix_present_flag > 0 {
                    let matrix_dim: usize = if out.chroma_format_idc != 2 { 8 } else { 12 };

                    for _ in 0..matrix_dim {
                        out.seq_scaling_list_present_flag.push(r.read_u8(1)?);
                    }
                }
            }
            _ => {}
        }

        Ok(out)
    }
}

pub async fn write_frames(
    session: retina::client::Session<retina::client::Described>,
    stop_signal: std::pin::Pin<Box<dyn futures::Future<Output = Result<(), std::io::Error>>>>,
) -> Result<()> {
    use ffmpeg_next as ffmpeg;
    ffmpeg_next::init()?;

    tokio::pin!(stop_signal);

    let mut session = session
        .play(retina::client::PlayOptions::default())
        .await?
        .demuxed()?;

    // let mut frame;

    debug!("waiting for first frame");
    let params = loop {
        let f0 = futures::StreamExt::next(&mut session)
            .await
            .unwrap()
            .unwrap();

        match &f0 {
            retina::codec::CodecItem::VideoFrame(f) => {
                // debug!("got frame: {:?}", f);
                let stream = &session.streams()[f.stream_id()];
                let start_ctx = *f.start_ctx();

                let params = stream.parameters().unwrap();

                debug!("stream.media_type: {:?}", stream.media());
                debug!("stream.encoding_name: {:?}", stream.encoding_name());

                match params {
                    retina::codec::ParametersRef::Video(ps) => {
                        // debug!("video params: {:#?}", ps);

                        let mut extra = ps.extra_data().to_vec();
                        debug!("extra data: {:?}", extra.len());

                        // let sps = Sps::parse_from(&extra)?;

                        // debug!("sps: {:#?}", sps);

                        decode_avc_decoder_config(&extra)?;

                        /// frame = f0;
                        break ps;
                    }
                    retina::codec::ParametersRef::Audio(_) => todo!(),
                    retina::codec::ParametersRef::Message(_) => todo!(),
                }
            }
            _ => warn!("unexpected item"),
        }
    };

    Ok(())
}

fn decode_avc_decoder_config(data: &[u8]) -> Result<h264_reader::nal::sps::SeqParameterSet> {
    use bytes::Buf;
    use h264_reader::nal::Nal;

    debug!("\n{}", pretty_hex::pretty_hex(&data));
    debug!("data.len: {:?}", data.len());

    // let mut r = bitreader::BitReader::new(&data);
    let mut r = std::io::Cursor::new(data);

    // let version = r.read_u8(8)?;
    let version = r.get_u8();
    assert_eq!(version, 0x01);

    let profile_idc = r.get_u8();
    let profile_compatibility = r.get_u8();
    let level_idc = r.get_u8();

    let length_size_minus_one = r.get_u8();
    assert_eq!(length_size_minus_one, 0xff);

    assert_eq!(r.get_u8(), 0xe1);

    let sps_nal_len = r.get_u16();
    debug!("sps_nal_len: {:?}", sps_nal_len);
    let sps_nal = r.copy_to_bytes(sps_nal_len as usize);

    assert_eq!(r.get_u8(), 0x01); // number of PPSs
    let pps_nal_len = r.get_u16();
    debug!("pps_nal_len: {:?}", pps_nal_len);
    let pps_nal = r.copy_to_bytes(pps_nal_len as usize);

    let sps = h264_reader::rbsp::decode_nal(&sps_nal)?;
    let sps_nal = h264_reader::nal::RefNal::new(&sps_nal, &[], true);
    assert!(sps_nal.is_complete());

    let sps = h264_reader::nal::sps::SeqParameterSet::from_bits(sps_nal.rbsp_bits()).unwrap();

    debug!("sps: {:#?}", sps);

    // let pps = h264_reader::rbsp::decode_nal(&pps_nal)?;
    // let pps_nal = h264_reader::nal::RefNal::new(&pps_nal, &[], true);
    // assert!(pps_nal.is_complete());
    // let pps = h264_reader::nal::pps::PicParameterSet::from_bits(pps_nal.rbsp_bits());
    // debug!("pps: {:#?}", pps);

    Ok(sps)
}

#[cfg(feature = "nope")]
pub async fn write_frames(
    session: retina::client::Session<retina::client::Described>,
    stop_signal: std::pin::Pin<Box<dyn futures::Future<Output = Result<(), std::io::Error>>>>,
) -> Result<()> {
    use ffmpeg_next as ffmpeg;
    ffmpeg_next::init()?;

    tokio::pin!(stop_signal);

    let mut session = session
        .play(retina::client::PlayOptions::default())
        .await?
        .demuxed()?;

    /// ffmpeg setup
    let codec: ffmpeg_next::Codec = ffmpeg::decoder::find(ffmpeg::codec::Id::H264).unwrap();

    let context_decoder = ffmpeg::codec::context::Context::new_with_codec(codec);
    debug!("getting decoder");
    let mut decoder: ffmpeg_next::decoder::Video = context_decoder.decoder().video().unwrap();
    debug!("got decoder");

    // debug!("decoder.format: {:?}", decoder.format());
    // debug!("decoder.width: {:?}", decoder.width());
    // debug!("decoder.height: {:?}", decoder.height());

    let mut frame;

    debug!("waiting for first frame");
    let params = loop {
        let f0 = futures::StreamExt::next(&mut session)
            .await
            .unwrap()
            .unwrap();

        match &f0 {
            retina::codec::CodecItem::VideoFrame(f) => {
                // debug!("got frame: {:?}", f);
                let stream = &session.streams()[f.stream_id()];
                let start_ctx = *f.start_ctx();

                let params = stream.parameters().unwrap();

                debug!("stream.media_type: {:?}", stream.media());
                debug!("stream.encoding_name: {:?}", stream.encoding_name());

                match params {
                    retina::codec::ParametersRef::Video(ps) => {
                        // debug!("video params: {:#?}", ps);

                        let mut extra = ps.extra_data().to_vec();
                        debug!("extra data: {:?}", extra.len());

                        // let sps = Sps::parse_from(&extra)?;

                        // debug!("sps: {:#?}", sps);

                        frame = f0;
                        break ps;
                    }
                    retina::codec::ParametersRef::Audio(_) => todo!(),
                    retina::codec::ParametersRef::Message(_) => todo!(),
                }
            }
            _ => warn!("unexpected item"),
        }
    };

    debug!("setting decoder params");
    /// SAFETY: unsure?
    unsafe {
        (*decoder.as_mut_ptr()).height = params.pixel_dimensions().1 as i32;
        (*decoder.as_mut_ptr()).width = params.pixel_dimensions().0 as i32;
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

    debug!("starting loop");
    loop {
        match frame {
            retina::codec::CodecItem::VideoFrame(f) => {
                let stream = &session.streams()[f.stream_id()];
                let start_ctx = *f.start_ctx();

                // let data = f.data();
                let mut data = Vec::new();

                if !f.data().starts_with(&[0x00, 0x00, 0x00, 0x01]) {
                    /// Add start code
                    data.extend_from_slice(&[0x00, 0x00, 0x00, 0x01]);
                }
                /// If it's a fragmented NAL unit, you might need additional processing here
                /// For simplicity, we're assuming complete NAL units
                data.extend_from_slice(f.data());

                debug!("frame len: {:?}", data.len());

                let packet = ffmpeg::Packet::copy(&data);

                decoder.send_packet(&packet)?;

                let mut decoded_frame = ffmpeg::util::frame::video::Video::empty();
                while decoder.receive_frame(&mut decoded_frame).is_ok() {
                    // Process the decoded frame
                    debug!("Decoded a frame");
                    // You might want to do something with the decoded frame here
                }

                debug!("breaking");

                break;
            }
            retina::codec::CodecItem::MessageFrame(msg) => {
                info!("message: {:?}", msg);
            }
            _ => warn!("unexpected item"),
        }

        debug!("waiting for frame");
        let f = futures::StreamExt::next(&mut session)
            .await
            .unwrap()
            .unwrap();

        frame = f;

        debug!("got frame");
    }

    #[cfg(feature = "nope")]
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

                let data = f.data();
                // let mut data = Vec::new();

                // /// Add start code
                // data.extend_from_slice(&[0x00, 0x00, 0x00, 0x01]);

                // /// If it's a fragmented NAL unit, you might need additional processing here
                // /// For simplicity, we're assuming complete NAL units
                // data.extend_from_slice(f.data());

                debug!("frame len: {:?}", data.len());

                let packet = ffmpeg::Packet::copy(&data);

                decoder.send_packet(&packet)?;

                break;
            }
            retina::codec::CodecItem::MessageFrame(msg) => {
                info!("message: {:?}", msg);
            }
            _ => warn!("unexpected item"),
        }
    }

    Ok(())
}

#[cfg(feature = "nope")]
pub async fn write_frames2(
    session: retina::client::Session<retina::client::Described>,
    stop_signal: std::pin::Pin<Box<dyn futures::Future<Output = Result<(), std::io::Error>>>>,
) -> Result<()> {
    use ffmpeg_next as ffmpeg;
    ffmpeg_next::init()?;

    tokio::pin!(stop_signal);

    let mut session = session.play(retina::client::PlayOptions::default()).await?;

    /// ffmpeg setup
    let codec = ffmpeg::decoder::find(ffmpeg::codec::Id::H264).unwrap();

    let context_decoder = ffmpeg::codec::context::Context::new_with_codec(codec);
    debug!("getting decoder");
    let mut decoder: ffmpeg_next::decoder::Video = context_decoder.decoder().video().unwrap();
    debug!("got decoder");

    // /// doesn't work?
    // let context_decoder: ffmpeg_next::Codec =
    //     ffmpeg_next::codec::traits::Decoder::decoder(codec).unwrap();
    // debug!("getting decoder");
    // let mut decoder: ffmpeg_next::codec::Video = context_decoder.video()?;
    // debug!("got decoder");

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

pub fn read_uev(bit_reader: &mut bitreader::BitReader) -> Result<u32> {
    let mut leading_zeros_bits: u8 = 0;

    loop {
        if bit_reader.read_u8(1)? != 0 {
            break;
        }
        leading_zeros_bits += 1;
    }
    let code_num = (1 << leading_zeros_bits) - 1 + bit_reader.read_u64(leading_zeros_bits)?;
    Ok(code_num as u32)
}

#[cfg(feature = "nope")]
async fn write_frames2(
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
