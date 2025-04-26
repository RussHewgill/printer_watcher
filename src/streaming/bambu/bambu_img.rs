use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use rumqttc::tokio_rustls::{self, rustls};
use std::sync::Arc;
use tokio::{io::AsyncReadExt, sync::RwLock};

use crate::config::printer_id::PrinterId;

/// https://github.com/greghesp/ha-bambulab/blob/main/custom_components/bambu_lab/pybambu/bambu_client.py#L68
pub struct JpegStreamViewer {
    auth_data: Vec<u8>,
    tls_stream: tokio_rustls::client::TlsStream<tokio::net::TcpStream>,
    buf: [u8; Self::READ_CHUNK_SIZE],
    handle: egui::TextureHandle,
    kill_rx: tokio::sync::mpsc::UnboundedReceiver<()>,
    last_rx: std::time::Instant,
    // msg_tx: tokio::sync::mpsc::UnboundedSender<()>,
    id: PrinterId,
    started: std::time::Instant,
}

/// consts
impl JpegStreamViewer {
    const JPEG_START: [u8; 4] = [0xff, 0xd8, 0xff, 0xe0];
    const JPEG_END: [u8; 2] = [0xff, 0xd9];
    const READ_CHUNK_SIZE: usize = 4096;

    const MAX_STREAM_LIFETIME_SEC: u64 = 60 * 10; // 10 minutes

    const STREAM_TIMEOUT: u64 = 10;
}

/// working
// #[cfg(feature = "nope")]
impl JpegStreamViewer {
    pub async fn new(
        id: PrinterId,
        serial: String,
        host: String,
        access_code: String,
        handle: egui::TextureHandle,
        kill_rx: tokio::sync::mpsc::UnboundedReceiver<()>,
        // msg_tx: tokio::sync::mpsc::UnboundedSender<()>,
    ) -> Result<Self> {
        let addr = format!("{}:6000", host);

        let client_config = rustls::ClientConfig::builder()
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(
                crate::conn_manager::conn_bambu::bambu_proto::NoCertificateVerification {},
            ))
            .with_no_client_auth();

        let connector = rumqttc::tokio_rustls::TlsConnector::from(Arc::new(client_config));

        // debug!("Jpeg Viewer Connecting");
        let stream = tokio::net::TcpStream::connect(addr).await?;
        // debug!("Jpeg Viewer Connected");

        let domain = rustls::pki_types::ServerName::try_from(host).unwrap();
        let mut tls_stream = connector.connect(domain, stream).await?;
        // debug!("TLS handshake completed");

        let auth_data = {
            use byteorder::{LittleEndian, WriteBytesExt};

            let username = "bblp";

            let mut auth_data = vec![];
            auth_data.write_u32::<LittleEndian>(0x40).unwrap();
            auth_data.write_u32::<LittleEndian>(0x3000).unwrap();
            auth_data.write_u32::<LittleEndian>(0).unwrap();
            auth_data.write_u32::<LittleEndian>(0).unwrap();

            for &b in username.as_bytes() {
                auth_data.push(b);
            }
            for _ in 0..(32 - username.len()) {
                auth_data.push(0);
            }

            for &b in access_code.as_bytes() {
                auth_data.push(b);
            }
            for _ in 0..(32 - access_code.len()) {
                auth_data.push(0);
            }
            auth_data
        };

        Ok(Self {
            auth_data,
            tls_stream,
            buf: [0u8; Self::READ_CHUNK_SIZE],
            handle,
            kill_rx,
            last_rx: std::time::Instant::now(),
            // msg_tx,
            id,
            started: std::time::Instant::now(),
        })
    }

    pub async fn run(&mut self) -> Result<()> {
        // // check if last rx was too long ago
        // let elapsed = self.last_rx.elapsed().as_secs();
        // if elapsed > 15 {
        //     crate::notifications::alert_printer_stream_error(
        //         &self.id,
        //         "Stream has been inactive for too long",
        //     )
        //     .await;
        //     error!("stream has been inactive for {} seconds", elapsed);
        //     bail!("stream has been inactive for {} seconds", elapsed);
        // }

        tokio::io::AsyncWriteExt::write_all(&mut self.tls_stream, &self.auth_data).await?;

        // debug!("getting socket status");
        let status = self.tls_stream.get_ref().0.take_error();
        if !matches!(status, Ok(None)) {
            error!("socket status = {:?}", status);
            bail!("socket status = {:?}", status);
        }
        // debug!("socket status ok, running loop");

        let mut payload_size = 0;
        let mut img_buf: Vec<u8> = vec![];
        let mut got_header = false;

        loop {
            if self.started.elapsed().as_secs() > Self::MAX_STREAM_LIFETIME_SEC {
                error!("stream has been running for too long, exiting");
                break;
            }

            // debug!("looping");
            self.buf.fill(0);

            let n = match tokio::time::timeout(
                tokio::time::Duration::from_secs(Self::STREAM_TIMEOUT),
                self.tls_stream.read(&mut self.buf),
            )
            .await
            {
                Ok(n) => n?,
                Err(_) => {
                    warn!("timeout reading from stream");
                    // self.msg_tx.send(StreamMsg::Panic(self.id.clone())).unwrap();
                    bail!("timeout reading from stream");
                }
            };
            // let n = self.tls_stream.read(&mut self.buf).await?;
            // debug!("got {} bytes", n);

            if got_header {
                // debug!("extending image by {}", n);
                img_buf.extend_from_slice(&self.buf[..n]);

                if img_buf.len() > payload_size {
                    warn!(
                        "unexpected image payload received: {} > {}",
                        img_buf.len(),
                        payload_size,
                    );
                    // got_header = false;
                    // img_buf.clear();

                    // /// not sure what the extra data is?
                    // img_buf.truncate(payload_size);

                    bail!(
                        "unexpected image payload received: {} > {}",
                        img_buf.len(),
                        payload_size,
                    );

                    // break;
                }
                if img_buf.len() == payload_size {
                    if &img_buf[0..4] != &Self::JPEG_START {
                        warn!("missing jpeg start bytes");
                        break;
                    } else if &img_buf[payload_size - 2..payload_size - 0] != &Self::JPEG_END {
                        warn!("missing jpeg end bytes");
                        break;
                    }

                    // debug!("got image");
                    /// use image crate to write jpeg to file
                    // let mut f = std::fs::File::create("test.jpg")?;
                    // std::io::Write::write_all(&mut f, &img)?;
                    let image = match image::load_from_memory(&img_buf) {
                        Ok(image) => image,
                        Err(e) => {
                            error!("failed to load image: {}", e);
                            break;
                        }
                    };
                    let img_size = [image.width() as _, image.height() as _];
                    let image_buffer = image.to_rgba8();
                    let pixels = image_buffer.as_flat_samples();
                    let img = egui::ColorImage::from_rgba_unmultiplied(img_size, pixels.as_slice());

                    self.handle.set(img, Default::default());

                    got_header = false;
                    img_buf.clear();

                    self.last_rx = std::time::Instant::now();
                }
            } else if n == 16 {
                // debug!("got header");
                // img.extend_from_slice(&buf);

                // payload_size = int.from_bytes(dr[0:3], byteorder='little')
                // payload_size = u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]) as usize;
                payload_size =
                    <byteorder::LittleEndian as byteorder::ByteOrder>::read_u32(&self.buf[0..4])
                        as usize;

                // debug!("payload_size = {}", payload_size);
                got_header = true;
            }

            if n == 0 {
                debug!("wrong access code");
                break;
            }
        }

        warn!("Exiting stream loop");

        Ok(())
    }
}
