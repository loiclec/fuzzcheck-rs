use decent_serde_json_alternative::{FromJson, ToJson};

use std::io::prelude::*;
use std::net::TcpStream;
use crate::{FuzzerEvent, FuzzerStats};

pub fn write(stream: &mut TcpStream, message: &str) {
    let bytes = message.as_bytes();
    let len = bytes.len() as u32;
    let be_len = len.to_be_bytes();

    let mut message = be_len.to_vec();
    message.extend(bytes);

    stream.write_all(&message).unwrap();

    stream.flush().unwrap();
}

pub fn read(stream: &mut TcpStream) -> Option<String> {
    let mut be_len = [0u8; std::mem::size_of::<u32>()];
    stream.read_exact(&mut be_len).ok()?;

    let len = u32::from_be_bytes(be_len);
    let mut buffer = std::iter::repeat(0u8).take(len as usize).collect::<Box<[_]>>();
    stream.read_exact(&mut buffer).ok()?;

    Some(String::from_utf8_lossy(&buffer).to_string())
}

#[derive(Clone, Copy, FromJson, ToJson)]
pub enum MessageUserToFuzzer {
    Pause,
    UnPause,
    UnPauseUntilNextEvent,
    Stop,
}

#[derive(Clone, FromJson, ToJson)]
pub enum TuiMessage {
    AddInput { hash: String, input: String },
    RemoveInput { hash: String, input: String },
    SaveArtifact { hash: String, input: String },
    ReportEvent(TuiMessageEvent),
    Paused,
    UnPaused,
    Stopped,
}

#[derive(Clone, FromJson, ToJson)]
pub struct TuiMessageEvent {
    pub event: FuzzerEvent,
    pub stats: FuzzerStats,
    pub time_ms: usize,
}
