use std::fmt::Display;

use color_eyre::eyre::Result;
use serde_derive::{Deserialize, Serialize};

use crate::error::AsEyreErrorResult;

#[derive(Debug, Serialize, Deserialize)]
pub struct PlayReq;

#[derive(Debug, Serialize, Deserialize)]
pub enum Request {
    Play(PlayReq),
    Shutdown,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Response {
    Ok,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Message {
    Request(Request),
    Response(Response),
}

pub struct ServerRequest(pub Request, pub ConnId, pub ws::Sender);

impl std::fmt::Debug for ServerRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ServerRequest({:?}, {:?}, ws::Sender {{ .. }})", self.0, self.1)
    }
}

/////////////////
// Connections //
/////////////////

#[derive(Debug, Eq, PartialEq, Hash, Copy, Clone, Ord, PartialOrd, Serialize, Deserialize)]
pub struct ConnId(u64);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Address {
    pub host: String,
    pub port: u16,
}

impl Display for Address {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.host, self.port)
    }
}

#[derive(Debug)]
pub enum WSEvent {
    Timeout,
    Shutdown,
    Close(ws::CloseCode, String),
    Error(ws::Error),
}

#[derive(Debug)]
pub enum WSMsg {
    Open,
    Message(Message),
    Timeout,
    Shutdown,
    Close(ws::CloseCode, String),
    Error(ws::Error),
    InitError(ws::Error),
}

pub(crate) fn get_ws_builder(max_connections: usize) -> ws::Builder {
    let mut builder = ws::Builder::new();
    builder.with_settings(ws::Settings {
        max_connections,
        tcp_nodelay: true,
        ..Default::default()
    });

    builder
}

pub(crate) fn send_json_message(message: &Message, sender: &ws::Sender) -> Result<()> {
    let serialized = serde_json::to_string(&message).unwrap_or_else(|e| {
        panic!("to_string failed on \"{}\" with {:?} as input", e, message);
    });
    sender.send(serialized).as_eyre_result()
}
