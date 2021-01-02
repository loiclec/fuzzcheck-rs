use std::{
    net::{TcpListener, TcpStream},
    process::Stdio,
    rc::Rc,
};

use fuzzcheck_common::arg::FullCommandLineArguments;

use crate::project::Root;

use super::fuzz_target_comm;

pub struct FuzzingView {
    _listener: TcpListener,
    _stream: TcpStream,
}

// impl FuzzingView {
//     fn new(root: Rc<Root>, mut args: ResolvedCommandLineArguments) -> Self {
//         let (listener, sock_addr) = fuzz_target_comm::create_listener();
//         args.socket_address = Some(sock_addr);

//         let child_process = root.run_command(args, || Stdio::null());

//         let stream = fuzz_target_comm::accept(listener);
//     }
// }
