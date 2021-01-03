use std::sync::mpsc;
use std::thread;
use std::{io, time::Duration};

use termion::event::Key;
use termion::input::TermRead;
use thread::JoinHandle;

pub const EXIT_KEY: Key = Key::Ctrl('c');

pub enum Event<S>
where
    S: Send + 'static,
{
    UserInput(Key),
    Subscription(S),
    Tick,
}

pub struct Events<S>
where
    S: Send + 'static,
{
    pub tx: mpsc::Sender<Event<S>>,
    rx: mpsc::Receiver<Event<S>>,
    handles: Vec<JoinHandle<()>>,
}

impl<S> Events<S>
where
    S: Send + 'static,
{
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        let input_handle = {
            let tx = tx.clone();

            std::thread::spawn(move || {
                let stdin = io::stdin();
                for evt in stdin.keys() {
                    if let Ok(key) = evt {
                        if let Err(_) = tx.send(Event::UserInput(key)) {
                            return;
                        }
                        if key == EXIT_KEY {
                            return;
                        }
                    }
                }
            })
        };

        let tick_handle = {
            let tx = tx.clone();

            std::thread::spawn(move || loop {
                std::thread::sleep(Duration::from_millis(1000));
                if let Err(_) = tx.send(Event::Tick) {
                    return;
                }
            })
        };

        Events {
            tx,
            rx,
            handles: vec![input_handle, tick_handle],
        }
    }

    pub fn add_stream(&mut self, stream: impl FnOnce(mpsc::Sender<Event<S>>) + Send + 'static) {
        let tx = self.tx.clone();
        self.handles.push(thread::spawn(move || stream(tx)));
    }

    pub fn next(&self) -> Result<Event<S>, mpsc::RecvError> {
        self.rx.recv()
    }
}
