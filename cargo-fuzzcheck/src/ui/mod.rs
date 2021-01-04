mod framework;
mod hlist;
mod horizontal_list_view;
mod text_field_view;
mod vertical_list_view;

mod error_view;
mod fuzzing;
mod initialized;
mod preinit;
mod run_fuzz;

mod app;
mod events;
mod fuzz_target_comm;

use std::{cell::RefCell, error::Error, net::TcpStream, path::PathBuf, rc::Rc, sync::{Arc, Mutex}};
use std::{
    io::{self, Stdout},
    process::Stdio,
};

use events::EXIT_KEY;
use fuzz_target_comm::send_fuzzer_message;
use fuzzcheck_common::ipc::TuiMessage;
use fuzzing::FuzzingView;

use crate::ui::framework::ViewState;

// use comm::FuzzingEvent;
use termion::raw::IntoRawMode;
use termion::screen::AlternateScreen;
use termion::{input::MouseTerminal, raw::RawTerminal};

use tui::{backend::TermionBackend, Terminal};

use self::framework::Theme;

type TerminalAlias = Terminal<TermionBackend<AlternateScreen<MouseTerminal<RawTerminal<Stdout>>>>>;

fn set_ui_terminal(raw: bool) -> Result<TerminalAlias, Box<dyn Error>> {
    let stdout = io::stdout().into_raw_mode()?;
    if raw {
        stdout.activate_raw_mode()?
    } else {
        stdout.suspend_raw_mode()?
    };
    let stdout = MouseTerminal::from(stdout);
    let stdout = AlternateScreen::from(stdout);
    let backend = TermionBackend::new(stdout);
    Ok(Terminal::new(backend)?)
}

pub fn launch_app(root_path: PathBuf) -> Result<(), Box<dyn Error>> {
    // Terminal initialization
    let mut terminal = set_ui_terminal(true)?;

    let mut events = events::Events::<TuiMessage>::new();

    let mut state = app::State::new(root_path, events.tx.clone());

    let mut child_process: Option<std::process::Child> = None;
    let mut sending_stream = Option::<TcpStream>::None;

    'main_loop: loop {
        terminal.draw(|f| {
            state.draw(f, &Theme::primary(), f.size());
        })?;

        // blocking
        let event = events.next()?;

        // first check if the event needs to be intercepted,
        // for example if it's the exit key
        match &event {
            events::Event::UserInput(key) => {
                if key == &EXIT_KEY {
                    break 'main_loop;
                }
            }
            _ => {}
        }

        if let Some(update) = state.convert_in_message(event) {
            if let Some(out_message) = state.update(update) {
                match out_message {
                    app::OutMessage::Quit => {
                        break 'main_loop;
                    }
                    app::OutMessage::StartFuzzing {
                        root,
                        target_name,
                        config,
                    } => {
                        if child_process.is_none() {
                            // Terminal initialization
                            terminal.clear()?;
                            std::mem::drop(terminal);

                            root.build_command(target_name.as_ref(), &config, &Stdio::inherit)
                                .unwrap();

                            let (listener, socket_address) = fuzz_target_comm::create_listener();

                            let out = state.update(app::Update::ChangePhase(app::Phase::Fuzzing(FuzzingView::new())));
                            assert!(out.is_none());

                            let mut config = config;
                            config.socket_address = Some(socket_address);

                            let child = root
                                .launch_executable(target_name.as_ref(), &config, &Stdio::null)
                                .unwrap();
                            child_process = Some(child);


                            let read_stream = fuzz_target_comm::accept(listener);
                            sending_stream = Some(read_stream.try_clone().unwrap());
                            
                            events.add_stream(move |tx| fuzz_target_comm::receive_fuzz_target_messages(read_stream, tx));
                            
                            terminal = set_ui_terminal(true)?;
                        } else {
                            panic!()
                        }
                    }
                    app::OutMessage::PauseFuzzer => {
                        if let Some(stream) = &mut sending_stream {
                            send_fuzzer_message(stream, fuzzcheck_common::ipc::MessageUserToFuzzer::Pause)
                        }
                    }
                    app::OutMessage::UnPauseFuzzer => {
                        if let Some(stream) = &mut sending_stream {
                            send_fuzzer_message(stream, fuzzcheck_common::ipc::MessageUserToFuzzer::UnPause)
                        }
                    }
                }
            }
        } else {
        }
    }

    if let Some(child) = &mut child_process {
        child.kill()?;
    }

    Ok(())
}
