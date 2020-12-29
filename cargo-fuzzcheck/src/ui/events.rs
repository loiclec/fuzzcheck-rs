
use std::io;
use std::sync::mpsc;
use std::thread;

use termion::event::Key;
use termion::input::{TermRead};

use super::framework::UserInput;

pub const EXIT_KEY: Key = Key::Char('q');

pub enum Event<S> where S: Send + 'static {
    UserInput(UserInput),
    _Subscription(S),
}

pub struct Events<S> where S: Send + 'static {
    rx: mpsc::Receiver<Event<S>>,
    _input_handle: thread::JoinHandle<()>,
}

impl<S> Events<S> where S: Send + 'static {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        let input_handle = {
            let tx = tx.clone();

            std::thread::spawn(move || {
                let stdin = io::stdin();
                for evt in stdin.keys() {
                    if let Ok(key) = evt {
                        if let Err(_) = tx.send(Event::UserInput(UserInput::Key(key))) {
                            return;
                        }
                        if key == EXIT_KEY {
                            return;
                        }
                    }
                }
            })
        };

        Events {
            rx,
            _input_handle: input_handle,
        }
    }

    pub fn next(&self) -> Result<Event<S>, mpsc::RecvError> {
        self.rx.recv()
    }
}
