#[derive(Debug)]
pub enum WorkerMsg {

    // StatusUpdate(),

    Connecting,
    Connected,
    Reconnecting,
    Disconnected,
}

impl From<super::conn_bambu::message::Message> for WorkerMsg {
    fn from(value: super::conn_bambu::message::Message) -> Self {
        unimplemented!()
    }
}
