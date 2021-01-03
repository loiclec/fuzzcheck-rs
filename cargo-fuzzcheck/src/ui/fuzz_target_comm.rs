use std::net::SocketAddr;
use std::net::TcpListener;
use std::net::TcpStream;
use std::sync::mpsc::Sender;

use fuzzcheck_common::ipc;

use decent_serde_json_alternative::FromJson;
use ipc::TuiMessage;
use json;

use super::events::Event;

pub fn create_listener() -> (TcpListener, SocketAddr) {
    let server_listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = server_listener.local_addr().unwrap();
    (server_listener, addr)
}

pub fn receive_fuzz_target_messages(listener: TcpListener, tx: Sender<Event<TuiMessage>>) {
    let mut stream = listener.accept().unwrap().0;

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
}
