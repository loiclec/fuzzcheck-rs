use decent_serde_json_alternative::{FromJson, ToJson};

use fuzzcheck_common::ipc;
use std::net::TcpListener;
use std::net::{SocketAddr, TcpStream};
use std::sync::mpsc::Sender;

use ipc::{MessageUserToFuzzer, TuiMessage};
use json;

use super::events::Event;

pub fn create_listener() -> (TcpListener, SocketAddr) {
    let server_listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = server_listener.local_addr().unwrap();
    (server_listener, addr)
}

pub fn accept(listener: TcpListener) -> TcpStream {
    listener.accept().unwrap().0
}

pub fn receive_fuzz_target_messages(mut stream: TcpStream, tx: Sender<Event<TuiMessage>>) {
    let mut all_bytes = Vec::<u8>::new();

    loop {
        if let Some(s) = ipc::read(&mut stream) {
            let j = json::parse(&s).unwrap();
            let event = TuiMessage::from_json(&j).unwrap();
            tx.send(Event::Subscription(event)).unwrap();

            all_bytes.clear();
        } else {
            break;
        }
    }
    // TODO: end of subscription event?
}

pub fn send_fuzzer_message(stream: &mut TcpStream, message: MessageUserToFuzzer) {
    ipc::write(stream, json::stringify(message.to_json()).as_str());
}
