use std::{net::{TcpListener, TcpStream}, process::{Child, Stdio}, rc::Rc, sync::mpsc::Sender, thread};

use fuzzcheck_common::arg::FullCommandLineArguments;
use thread::JoinHandle;

use crate::project::{FullConfig, Root};

use super::{events::Event, fuzz_target_comm::{self, FuzzingEvent}};

pub struct FuzzingView {
    _child_process: Child,
    _thread_handle: JoinHandle<()>,
}

// impl FuzzingView {
//     fn new(root: Rc<Root>, target_name: &str, mut config: FullConfig, sender: Sender<Event<FuzzingEvent>>) -> Self {
//         let (listener, sock_addr) = fuzz_target_comm::create_listener();
//         config.socket_address = Some(sock_addr);
//         let _child_process = root.run_command(target_name, &config, &Stdio::null).unwrap();
//         let stream = fuzz_target_comm::accept(listener);

//         let thread_handle = thread::spawn(|| {
//             fuzz_target_comm::receive_fuzz_target_messages(stream, sender)
//         });

//         Self {
//             _child_process,
//             _thread_handle: thread_handle,
//         }
//     }
// }
