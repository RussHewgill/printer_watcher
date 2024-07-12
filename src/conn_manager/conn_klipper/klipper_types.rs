use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Rpc {
    pub jsonrpc: String,
    pub method: String,
    pub params: Vec<serde_json::Value>,
}

// pub struct
