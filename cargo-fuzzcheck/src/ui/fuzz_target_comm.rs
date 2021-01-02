use std::net::SocketAddr;
use std::net::TcpListener;
use std::net::TcpStream;
use std::sync::mpsc::Sender;

use fuzzcheck_common::ipc;

use decent_serde_json_alternative::FromJson;
use json;

#[derive(Debug, Clone, FromJson)]
pub enum FuzzingEvent {
    A,
}

pub fn create_listener() -> (TcpListener, SocketAddr) {
    let server_listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = server_listener.local_addr().unwrap();
    (server_listener, addr)
}

pub fn accept(listener: TcpListener) -> TcpStream {
    listener.accept().unwrap().0
}

pub fn receive_fuzz_target_messages(mut stream: TcpStream, tx: Sender<FuzzingEvent>) {
    let mut all_bytes = Vec::<u8>::new();

    loop {
        let s = ipc::read(&mut stream);
        let j = json::parse(&s).unwrap();
        let event = FuzzingEvent::from_json(&j).unwrap();

        tx.send(event).unwrap();

        all_bytes.clear();
    }
}
