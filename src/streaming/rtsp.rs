use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use processor::H264Processor;

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
                        ffmpeg::software::scaling::Flags::BILINEAR,
                    )
                    .unwrap()
                });
                let mut scaled = ffmpeg::util::frame::video::Video::empty();
                scaler.run(&decoded, &mut scaled)?;

                let mut buf = vec![];
                std::io::Write::write_all(
                    &mut buf,
                    format!("P6\n{} {}\n255\n", scaled.width(), scaled.height()).as_bytes(),
                )?;
                std::io::Write::write_all(&mut buf, scaled.data(0))?;

                debug!("getting image");
                let img = image::io::Reader::new(std::io::Cursor::new(buf))
                    .with_guessed_format()?
                    .decode()?;
                debug!("saving image");
                let filename = format!("frame{}.jpg", self.frame_i);
                img.save(filename)?;

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
