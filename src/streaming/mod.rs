use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use processor::H264Processor;

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

pub mod processor {
    use anyhow::{anyhow, bail, ensure, Context, Result};
    use ffmpeg_next as ffmpeg;
    use tracing::{debug, error, info, trace, warn};

    pub struct H264Processor {
        decoder: ffmpeg::codec::decoder::Video,
        scaler: Option<ffmpeg::software::scaling::Context>,
        frame_i: u64,
        convert_to_annex_b: bool,
    }

    impl H264Processor {
        pub fn new(convert_to_annex_b: bool) -> Self {
            let mut codec_opts = ffmpeg::Dictionary::new();
            if !convert_to_annex_b {
                codec_opts.set("is_avc", "1");
            }
            let codec = ffmpeg::codec::decoder::find(ffmpeg::codec::Id::H264).unwrap();
            let decoder = ffmpeg::codec::decoder::Decoder(ffmpeg::codec::Context::new())
                .open_as_with(codec, codec_opts)
                .unwrap()
                .video()
                .unwrap();
            Self {
                decoder,
                scaler: None,
                frame_i: 0,
                convert_to_annex_b,
            }
        }

        pub fn handle_parameters(
            &mut self,
            stream: &retina::client::Stream,
            p: &retina::codec::VideoParameters,
        ) -> Result<()> {
            if !self.convert_to_annex_b {
                let pkt = ffmpeg::codec::packet::Packet::borrow(p.extra_data());
                self.decoder.send_packet(&pkt)?;
            } else {
                // TODO: should convert and supply SPS/PPS, rather than relying on
                // them existing in-band within frames.
            }

            // let mut extra = p.extra_data().to_vec();
            // // debug!("extra data: {:?}", extra.len());
            // let (_, sps, pps) = super::decode_avc_decoder_config(&extra)?;

            // let mut extra = vec![];
            // extra.extend_from_slice(&[0x00, 0x00, 0x00, 0x01]);
            // extra.extend_from_slice(&sps);

            // extra.extend_from_slice(&[0x00, 0x00, 0x00, 0x01]);
            // extra.extend_from_slice(&pps);

            // self.decoder.set_parameters()
            // (*self.decoder.as_mut_ptr()).extradata

            // ffmpeg doesn't appear to actually handle the parameters until the
            // first full frame, so just note that the scaler needs to be
            // (re)created.
            self.scaler = None;
            Ok(())
        }

        pub fn send_frame(
            &mut self,
            stream: &retina::client::Stream,
            f: retina::codec::VideoFrame,
        ) -> Result<()> {
            // let data = convert_h264(f)?;
            let data = if self.convert_to_annex_b {
                convert_h264(stream, f)?
            } else {
                f.into_data()
            };
            let pkt = ffmpeg::codec::packet::Packet::borrow(&data);
            self.decoder.send_packet(&pkt)?;
            self.receive_frames()?;
            Ok(())
        }

        pub fn flush(&mut self) -> Result<()> {
            self.decoder.send_eof()?;
            self.receive_frames()?;
            Ok(())
        }

        fn receive_frames(&mut self) -> Result<()> {
            let mut decoded = ffmpeg::util::frame::video::Video::empty();
            loop {
                match self.decoder.receive_frame(&mut decoded) {
                    Err(ffmpeg::Error::Other {
                        errno: ffmpeg::util::error::EAGAIN,
                    }) => {
                        // No complete frame available.
                        break;
                    }
                    Err(e) => bail!(e),
                    Ok(()) => {}
                }

                // This frame writing logic lifted from ffmpeg-next's examples/dump-frames.rs.
                let scaler = self.scaler.get_or_insert_with(|| {
                    info!(
                        "image parameters: {:?}, {}x{}",
                        self.decoder.format(),
                        self.decoder.width(),
                        self.decoder.height()
                    );
                    ffmpeg::software::scaling::Context::get(
                        self.decoder.format(),
                        self.decoder.width(),
                        self.decoder.height(),
                        ffmpeg::format::Pixel::RGB24,
                        self.decoder.width(),
                        self.decoder.height(),
                        // 640,
                        // 360,
                        ffmpeg::software::scaling::Flags::BILINEAR,
                    )
                    .unwrap()
                    // self.decoder
                    //     .scaler(1920, 1080, ffmpeg::software::scaling::Flags::BILINEAR)
                    //     .unwrap()
                });
                let mut scaled = ffmpeg::util::frame::video::Video::empty();
                scaler.run(&decoded, &mut scaled)?;

                // let filename = format!("frame{}.jpg", self.frame_i);

                // let img = image::io::Reader::new()?.decode()?;

                // #[cfg(feature = "nope")]
                {
                    let filename = format!("frame{}.ppm", self.frame_i);
                    info!("writing {}", &filename);
                    let mut file = std::fs::File::create(&filename)?;
                    std::io::Write::write_all(
                        &mut file,
                        format!("P6\n{} {}\n255\n", scaled.width(), scaled.height()).as_bytes(),
                    )?;
                    // std::io::Write::write_all(&mut file, decoded.data(0))?;
                    std::io::Write::write_all(&mut file, scaled.data(0))?;

                    debug!("reading image");
                    let img = image::io::Reader::open(&filename)?.decode()?;
                    let filename2 = format!("frame{}.jpg", self.frame_i);
                    debug!("saving image");
                    img.save(filename2)?;
                    debug!("saved image");
                }

                self.frame_i += 1;
            }
            Ok(())
        }
    }

    /// https://github.com/scottlamb/retina/blob/main/examples/webrtc-proxy/src/main.rs#L310C1-L339C2
    fn convert_h264(
        stream: &retina::client::Stream,
        frame: retina::codec::VideoFrame,
    ) -> Result<Vec<u8>> {
        // TODO:
        // * For each IDR frame, copy the SPS and PPS from the stream's
        //   parameters, rather than depend on it being present in the frame
        //   already. In-band parameters aren't guaranteed.

        let mut data = frame.into_data();
        let mut i = 0;
        while i < data.len() - 3 {
            // Replace each NAL's length with the Annex B start code b"\x00\x00\x00\x01".
            let len = u32::from_be_bytes([data[i], data[i + 1], data[i + 2], data[i + 3]]) as usize;
            data[i] = 0;
            data[i + 1] = 0;
            data[i + 2] = 0;
            data[i + 3] = 1;
            i += 4 + len;
            if i > data.len() {
                bail!("partial NAL body");
            }
        }
        if i < data.len() {
            bail!("partial NAL length");
        }
        Ok(data)
    }

    #[cfg(feature = "nope")]
    /// Converts from AVC representation to the Annex B representation.
    fn convert_h264(data: &mut [u8]) -> Result<()> {
        let mut i = 0;
        while i < data.len() - 3 {
            // Replace each NAL's length with the Annex B start code b"\x00\x00\x00\x01".
            let len = u32::from_be_bytes([data[i], data[i + 1], data[i + 2], data[i + 3]]) as usize;
            data[i] = 0;
            data[i + 1] = 0;
            data[i + 2] = 0;
            data[i + 3] = 1;
            i += 4 + len;
            if i > data.len() {
                bail!("partial NAL body");
            }
        }
        if i < data.len() {
            bail!("partial NAL length");
        }
        Ok(())
    }
}

// #[cfg(feature = "nope")]
pub async fn write_frames(
    session: retina::client::Session<retina::client::Described>,
    stop_signal: std::pin::Pin<Box<dyn futures::Future<Output = Result<(), std::io::Error>>>>,
) -> Result<()> {
    use ffmpeg_next as ffmpeg;

    ffmpeg::init()?;
    ffmpeg::util::log::set_level(ffmpeg::util::log::Level::Trace);

    tokio::pin!(stop_signal);

    let mut session = session
        .play(retina::client::PlayOptions::default())
        .await?
        .demuxed()?;

    let video_stream_i = 0;

    // let mut processor = H264Processor::new(true);
    let mut processor = H264Processor::new(false);

    if let Some(retina::codec::ParametersRef::Video(v)) =
        session.streams()[video_stream_i].parameters()
    {
        debug!("initial parameters: {:#?}", v);
        processor.handle_parameters(&session.streams()[video_stream_i], v)?;
    }

    debug!("starting loop");
    loop {
        debug!("waiting for frame");
        let f = futures::StreamExt::next(&mut session)
            .await
            .unwrap()
            .unwrap();
        debug!("got frame");
        match f {
            retina::codec::CodecItem::VideoFrame(f) => {
                let stream = &session.streams()[f.stream_id()];
                let start_ctx = *f.start_ctx();
                if f.has_new_parameters() {
                    let v = match stream.parameters() {
                        Some(retina::codec::ParametersRef::Video(v)) => {
                            debug!("new parameters: {:#?}", v);
                            v
                        }
                        _ => unreachable!(),
                    };
                    processor.handle_parameters(stream, v)?;
                }
                processor.send_frame(stream, f)?;
                break;
            }
            retina::codec::CodecItem::MessageFrame(msg) => {
                info!("message: {:?}", msg);
            }
            _ => warn!("unexpected item"),
        }
    }

    processor.flush()?;
    Ok(())
}

/// demuxed
#[cfg(feature = "nope")]
pub async fn write_frames(
    session: retina::client::Session<retina::client::Described>,
    stop_signal: std::pin::Pin<Box<dyn futures::Future<Output = Result<(), std::io::Error>>>>,
) -> Result<()> {
    use ffmpeg_next as ffmpeg;

    ffmpeg::init()?;
    ffmpeg::util::log::set_level(ffmpeg::util::log::Level::Trace);

    tokio::pin!(stop_signal);

    let mut session = session
        .play(retina::client::PlayOptions::default())
        .await?
        .demuxed()?;

    /// ffmpeg setup
    // let codec: ffmpeg_next::Codec = ffmpeg::decoder::find(ffmpeg::codec::Id::H264).unwrap();
    // let context_decoder = ffmpeg::codec::context::Context::new_with_codec(codec);
    // debug!("getting decoder");
    // let mut decoder: ffmpeg_next::decoder::Video = context_decoder.decoder().video().unwrap();
    // debug!("got decoder");
    let mut codec_opts = ffmpeg::Dictionary::new();
    codec_opts.set("is_avc", "1");
    let codec = ffmpeg::codec::decoder::find(ffmpeg::codec::Id::H264).unwrap();
    let mut decoder = ffmpeg::codec::decoder::Decoder(ffmpeg::codec::Context::new())
        .open_as_with(codec, codec_opts)
        .unwrap()
        .video()
        .unwrap();

    #[cfg(feature = "nope")]
    {
        let mut frame;

        debug!("waiting for first frame");
        let (ps, sps) = loop {
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

                            let sps = decode_avc_decoder_config(&extra)?;

                            frame = f0;
                            break (ps, sps);
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
            (*decoder.as_mut_ptr()).height = ps.pixel_dimensions().1 as i32;
            (*decoder.as_mut_ptr()).width = ps.pixel_dimensions().0 as i32;
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

        debug!("starting loop");
        loop {
            match frame {
                retina::codec::CodecItem::VideoFrame(f) => {
                    debug!("got frame");
                    let stream = &session.streams()[f.stream_id()];
                    let start_ctx = *f.start_ctx();

                    // let data = f.data();
                    // let mut data = Vec::new();

                    // if !f.data().starts_with(&[0x00, 0x00, 0x00, 0x01]) {
                    //     /// Add start code
                    //     data.extend_from_slice(&[0x00, 0x00, 0x00, 0x01]);
                    // }
                    // /// If it's a fragmented NAL unit, you might need additional processing here
                    // /// For simplicity, we're assuming complete NAL units
                    // data.extend_from_slice(f.data());

                    // debug!("frame len: {:?}", data.len());

                    let packet = ffmpeg::Packet::copy(&f.data());

                    debug!("sending packet");
                    decoder.send_packet(&packet)?;
                    debug!("sent packet");

                    let mut decoded_frame = ffmpeg::util::frame::video::Video::empty();
                    // while decoder.receive_frame(&mut decoded_frame).is_ok() {
                    //     // Process the decoded frame
                    //     debug!("Decoded a frame");
                    //     // You might want to do something with the decoded frame here
                    //     debug!("breaking");
                    //     break;
                    // }

                    match decoder.receive_frame(&mut decoded_frame) {
                        Ok(()) => {
                            debug!("Decoded a frame");
                            debug!("breaking");
                            break;
                        }
                        Err(e) => warn!("error: {:?}", e),
                    }
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
    }

    Ok(())
}

fn decode_avc_decoder_config(
    data: &[u8],
) -> Result<(h264_reader::nal::sps::SeqParameterSet, Vec<u8>, Vec<u8>)> {
    use bytes::Buf;
    use h264_reader::nal::Nal;

    // debug!("\n{}", pretty_hex::pretty_hex(&data));
    // debug!("data.len: {:?}", data.len());

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
    // debug!("sps_nal_len: {:?}", sps_nal_len);
    let sps_nal_bytes = r.copy_to_bytes(sps_nal_len as usize);

    assert_eq!(r.get_u8(), 0x01); // number of PPSs
    let pps_nal_len = r.get_u16();
    // debug!("pps_nal_len: {:?}", pps_nal_len);
    let pps_nal_bytes = r.copy_to_bytes(pps_nal_len as usize);

    let sps = h264_reader::rbsp::decode_nal(&sps_nal_bytes)?;
    let sps_nal = h264_reader::nal::RefNal::new(&sps_nal_bytes, &[], true);
    assert!(sps_nal.is_complete());

    let sps = h264_reader::nal::sps::SeqParameterSet::from_bits(sps_nal.rbsp_bits()).unwrap();

    // debug!("sps: {:#?}", sps);

    // let pps = h264_reader::rbsp::decode_nal(&pps_nal)?;
    // let pps_nal = h264_reader::nal::RefNal::new(&pps_nal, &[], true);
    // assert!(pps_nal.is_complete());
    // let pps = h264_reader::nal::pps::PicParameterSet::from_bits(pps_nal.rbsp_bits());
    // debug!("pps: {:#?}", pps);

    Ok((sps, sps_nal_bytes.to_vec(), pps_nal_bytes.to_vec()))
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
