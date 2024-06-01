use anyhow::{anyhow, bail, ensure, Context, Result};
use rumqttc::Incoming;
use tracing::{debug, error, info, trace, warn};

use std::sync::Arc;
use tokio::sync::RwLock;

use crate::{
    config::{printer_config::PrinterConfigBambu, printer_id::PrinterId},
    conn_manager::conn_bambu::{command::Command, message::Message},
};

pub(super) struct BambuListener {
    pub(super) printer_cfg: Arc<RwLock<PrinterConfigBambu>>,
    pub(super) client: rumqttc::AsyncClient,
    pub(super) eventloop: rumqttc::EventLoop,
    pub(super) tx: tokio::sync::mpsc::UnboundedSender<(PrinterId, Message)>,
    pub(super) topic_device_report: String,
    pub(super) topic_device_request: String,
}

impl BambuListener {
    pub fn new(
        printer_cfg: Arc<RwLock<PrinterConfigBambu>>,
        client: rumqttc::AsyncClient,
        eventloop: rumqttc::EventLoop,
        tx: tokio::sync::mpsc::UnboundedSender<(PrinterId, Message)>,
        topic_device_report: String,
        topic_device_request: String,
    ) -> Self {
        Self {
            printer_cfg,
            client,
            eventloop,
            tx,
            topic_device_report,
            topic_device_request,
        }
    }

    /// MARK: main event handler
    pub(super) async fn poll_eventloop(&mut self) -> Result<()> {
        use rumqttc::Event;
        loop {
            let event = match self.eventloop.poll().await {
                Ok(event) => event,
                Err(e) => {
                    error!("Error in eventloop: {:?}", e);
                    continue;
                }
            };
            match event {
                Event::Outgoing(event) => {
                    // debug!("outgoing event: {:?}", event);
                }
                Event::Incoming(Incoming::PingResp) => {}
                Event::Incoming(Incoming::ConnAck(c)) => {
                    debug!("got ConnAck: {:?}", c.code);
                    if c.code == rumqttc::ConnectReturnCode::Success {
                        // debug!("Connected to MQTT");
                        self.client
                            .subscribe(&self.topic_device_report, rumqttc::QoS::AtMostOnce)
                            .await?;
                        debug!("sent subscribe to topic");
                        // self.send_pushall().await?;
                    } else {
                        error!("Failed to connect to MQTT: {:?}", c.code);
                    }
                }
                Event::Incoming(Incoming::SubAck(s)) => {
                    debug!("got SubAck");
                    if s.return_codes
                        .iter()
                        .any(|&r| r == rumqttc::SubscribeReasonCode::Failure)
                    {
                        error!("Failed to subscribe to topic");
                    } else {
                        debug!("sending pushall");
                        self.send_pushall().await?;
                        debug!("sent");
                        // debug!("sending get version");
                        // self.send_get_version().await?;
                        // debug!("sent");
                    }
                }
                Event::Incoming(Incoming::Publish(p)) => {
                    // debug!("incoming publish");
                    let msg = crate::conn_manager::conn_bambu::parse::parse_message(&p);
                    // debug!("incoming publish: {:?}", msg);
                    let id = self.printer_cfg.read().await.id.clone();
                    self.tx.send((id, msg))?;
                }
                Event::Incoming(event) => {
                    debug!("incoming other event: {:?}", event);
                }
            }
        }
    }

    pub(super) async fn send_get_version(&mut self) -> Result<()> {
        let payload = Command::GetVersion.get_payload();

        self.client
            .publish(
                &self.topic_device_request,
                rumqttc::QoS::AtMostOnce,
                false,
                payload,
            )
            .await?;

        Ok(())
    }

    pub(super) async fn send_pushall(&mut self) -> Result<()> {
        let command = Command::PushAll;
        let payload = command.get_payload();

        let qos = rumqttc::QoS::AtMostOnce;
        self.client
            .publish(&self.topic_device_request, qos, false, payload)
            .await?;

        Ok(())
    }
}
