mod framework;
mod hlist;
mod horizontal_list_view;
mod text_field_view;

mod error_view;
mod fuzzing;
mod initialized;
mod preinit;
mod run_fuzz;

mod app;
mod events;
mod fuzz_target_comm;

use std::io;
use std::{error::Error, path::PathBuf};

use events::EXIT_KEY;

use crate::ui::framework::ViewState;

// use comm::FuzzingEvent;
use termion::input::MouseTerminal;
use termion::raw::IntoRawMode;
use termion::screen::AlternateScreen;

use tui::{backend::TermionBackend, Terminal};

use self::{framework::Theme, fuzz_target_comm::FuzzingEvent};

pub fn launch_app(root_path: PathBuf) -> Result<(), Box<dyn Error>> {
    // Terminal initialization
    let stdout = io::stdout().into_raw_mode()?;
    let stdout = MouseTerminal::from(stdout);
    let stdout = AlternateScreen::from(stdout);
    let backend = TermionBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let events = events::Events::<FuzzingEvent>::new();

    let mut state = app::State::new(root_path, events.tx.clone());

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
                }
            }
        } else {
        }
    }

    Ok(())
}
