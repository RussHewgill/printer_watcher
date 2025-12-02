use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use futures::StreamExt;
use rumqttc::{
    tokio_rustls::{client, rustls},
    AsyncClient, EventLoop, Incoming, MqttOptions,
};
use std::{sync::Arc, time::Duration};
use tokio::sync::RwLock;

use crate::{
    auth::bambu_auth::AuthDb,
    config::{printer_config::PrinterConfigBambu, printer_id::PrinterId, AppConfig},
    conn_manager::{worker_message::WorkerMsg, WorkerCmd},
};

use super::{bambu_listener::BambuListener, command::Command, message::Message};

/// scary, insecure, do not allow outside of local network
#[derive(Debug)]
pub struct NoCertificateVerification {}

/// TODO: maybe at least check the serial is correct?
impl rumqttc::tokio_rustls::rustls::client::danger::ServerCertVerifier
    for NoCertificateVerification
{
    fn verify_server_cert(
        &self,
        end_entity: &rumqttc::tokio_rustls::rustls::pki_types::CertificateDer<'_>,
        intermediates: &[rumqttc::tokio_rustls::rustls::pki_types::CertificateDer<'_>],
        server_name: &rumqttc::tokio_rustls::rustls::pki_types::ServerName<'_>,
        ocsp_response: &[u8],
        now: rumqttc::tokio_rustls::rustls::pki_types::UnixTime,
    ) -> std::prelude::v1::Result<
        rumqttc::tokio_rustls::rustls::client::danger::ServerCertVerified,
        rumqttc::tokio_rustls::rustls::Error,
    > {
        // debug!("end_entity: {:?}", end_entity);
        // debug!("server_name: {:?}", server_name);
        // debug!("ocsp_response: {:?}", ocsp_response);
        Ok(rumqttc::tokio_rustls::rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &rustls::pki_types::CertificateDer<'_>,
        dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &rustls::pki_types::CertificateDer<'_>,
        dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<rumqttc::tokio_rustls::rustls::SignatureScheme> {
        vec![
            rustls::SignatureScheme::RSA_PKCS1_SHA1,
            rustls::SignatureScheme::ECDSA_SHA1_Legacy,
            rustls::SignatureScheme::RSA_PKCS1_SHA256,
            rustls::SignatureScheme::ECDSA_NISTP256_SHA256,
            rustls::SignatureScheme::RSA_PKCS1_SHA384,
            rustls::SignatureScheme::ECDSA_NISTP384_SHA384,
            rustls::SignatureScheme::RSA_PKCS1_SHA512,
            rustls::SignatureScheme::ECDSA_NISTP521_SHA512,
            rustls::SignatureScheme::RSA_PSS_SHA256,
            rustls::SignatureScheme::RSA_PSS_SHA384,
            rustls::SignatureScheme::RSA_PSS_SHA512,
            rustls::SignatureScheme::ED25519,
            rustls::SignatureScheme::ED448,
        ]
    }
}

pub struct BambuClient {
    config: Arc<RwLock<PrinterConfigBambu>>,
    client: rumqttc::AsyncClient,
    tx: tokio::sync::mpsc::UnboundedSender<(PrinterId, WorkerMsg)>,
    cmd_rx: tokio::sync::mpsc::UnboundedReceiver<WorkerCmd>,
    topic_device_request: String,
    topic_device_report: String,
}

impl BambuClient {
    pub async fn new_and_init(
        // auth: Arc<RwLock<AuthDb>>,
        config: AppConfig,
        printer_cfg: Arc<RwLock<PrinterConfigBambu>>,
        tx: tokio::sync::mpsc::UnboundedSender<(PrinterId, WorkerMsg)>,
        cmd_rx: tokio::sync::mpsc::UnboundedReceiver<WorkerCmd>,
        kill_rx: tokio::sync::oneshot::Receiver<()>,
    ) -> Result<Self> {
        if config.logged_in() {
            Self::_new_and_init_cloud(config.auth().clone(), printer_cfg, tx, cmd_rx, kill_rx).await
        } else {
            Self::_new_and_init_lan(printer_cfg, tx, cmd_rx, kill_rx).await
        }
    }

    async fn _new_and_init_cloud(
        auth: Arc<RwLock<AuthDb>>,
        printer_cfg: Arc<RwLock<PrinterConfigBambu>>,
        tx: tokio::sync::mpsc::UnboundedSender<(PrinterId, WorkerMsg)>,
        cmd_rx: tokio::sync::mpsc::UnboundedReceiver<WorkerCmd>,
        kill_rx: tokio::sync::oneshot::Receiver<()>,
    ) -> Result<Self> {
        debug!("init cloud mqtt listener");
        let client_id = format!("bambu-watcher-{}", nanoid::nanoid!(8));

        let (username, password) = {
            let db = auth.read().await;
            db.get_cloud_mqtt_creds()?
        };

        const CLOUD_HOST: &'static str = "us.mqtt.bambulab.com";

        let mut mqttoptions = rumqttc::MqttOptions::new(client_id, CLOUD_HOST, 8883);
        mqttoptions.set_keep_alive(Duration::from_secs(5));
        mqttoptions.set_credentials(&username, &password);

        let mut root_cert_store = rustls::RootCertStore::empty();
        root_cert_store.add_parsable_certificates(
            rustls_native_certs::load_native_certs().expect("could not load platform certs"),
        );

        let client_config = rustls::ClientConfig::builder()
            .with_root_certificates(root_cert_store)
            .with_no_client_auth();

        let transport = rumqttc::Transport::tls_with_config(rumqttc::TlsConfiguration::Rustls(
            Arc::new(client_config),
        ));

        mqttoptions.set_transport(transport);
        // mqttoptions.set_clean_session(true);

        debug!("connecting, printer = {}", &printer_cfg.read().await.name);
        let (mut client, mut eventloop) = AsyncClient::new(mqttoptions, 10);
        debug!("connected, printer = {}", &printer_cfg.read().await.name);

        let mut out = Self {
            config: printer_cfg.clone(),
            client,
            tx,
            cmd_rx,
            topic_device_request: format!("device/{}/request", &printer_cfg.read().await.serial),
            topic_device_report: format!("device/{}/report", &printer_cfg.read().await.serial),
        };

        out.init(eventloop, kill_rx).await?;

        Ok(out)
    }

    async fn _new_and_init_lan(
        printer_cfg: Arc<RwLock<PrinterConfigBambu>>,
        tx: tokio::sync::mpsc::UnboundedSender<(PrinterId, WorkerMsg)>,
        cmd_rx: tokio::sync::mpsc::UnboundedReceiver<WorkerCmd>,
        kill_rx: tokio::sync::oneshot::Receiver<()>,
    ) -> Result<Self> {
        debug!("init lan mqtt listener");
        let client_id = format!("bambu-watcher-{}", nanoid::nanoid!(8));

        let printer = printer_cfg.read().await;

        if printer.host.is_empty() {
            bail!("missing host");
        }

        let mut mqttoptions = MqttOptions::new(client_id, &printer.host, 8883);
        mqttoptions.set_keep_alive(Duration::from_secs(5));
        mqttoptions.set_credentials("bblp", &printer.access_code);

        let client_config = rumqttc::tokio_rustls::rustls::ClientConfig::builder()
            // .with_root_certificates(root_cert_store)
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(NoCertificateVerification {}))
            .with_no_client_auth();

        let transport = rumqttc::Transport::tls_with_config(rumqttc::TlsConfiguration::Rustls(
            Arc::new(client_config),
        ));

        mqttoptions.set_transport(transport);
        // mqttoptions.set_clean_session(true);

        mqttoptions.set_max_packet_size(100 * 1024, 100 * 1024);

        debug!("connecting, printer = {}", &printer.name);
        let (mut client, mut eventloop) = AsyncClient::new(mqttoptions, 10);
        debug!("connected, printer = {}", &printer.name);

        let mut out = Self {
            config: printer_cfg.clone(),
            client,
            tx,
            cmd_rx,
            topic_device_request: format!("device/{}/request", &printer.serial),
            topic_device_report: format!("device/{}/report", &printer.serial),
        };
        out.init(eventloop, kill_rx).await?;

        Ok(out)
    }

    pub async fn init(
        &mut self,
        eventloop: EventLoop,
        mut kill_rx: tokio::sync::oneshot::Receiver<()>,
    ) -> Result<()> {
        let config2 = self.config.clone();
        let client2 = self.client.clone();
        let tx2 = self.tx.clone();
        let topic_report = self.topic_device_report.clone();
        let topic_request = self.topic_device_request.clone();

        // if let Err(e) = self.publish(Command::GetVersion).await {
        //     error!("Error publishing command: {:?}", e);
        // }

        tokio::task::spawn(async move {
            let mut listener = BambuListener::new(
                config2,
                client2,
                eventloop,
                tx2,
                topic_report,
                topic_request,
            );

            loop {
                tokio::select! {
                    _ = &mut kill_rx => {
                        debug!("Listener task got kill command");
                        break;
                    }
                    event = listener.poll_eventloop() => {
                        if let Err(e) = event {
                            error!("Error in listener: {:?}", e);
                            listener
                                .tx
                                .send((
                                    listener.printer_cfg.read().await.id.clone(),
                                    // Message::Disconnected,
                                    WorkerMsg::Disconnected,
                                ))
                                .unwrap();
                        }
                        listener.eventloop.clean();
                        debug!("Reconnecting...");
                    }
                }
            }
        });
        Ok(())
    }

    pub async fn publish(&self, command: Command) -> Result<()> {
        let payload = command.get_payload();

        let qos = rumqttc::QoS::AtMostOnce;
        self.client
            .publish(&self.topic_device_request, qos, false, payload)
            .await?;

        Ok(())
    }
}
