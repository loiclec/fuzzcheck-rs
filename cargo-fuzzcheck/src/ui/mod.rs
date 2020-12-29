
mod framework;

mod error_view;
mod preinit;
mod initialized;

mod events;
mod fuzz_target_comm;
mod app;

use std::{error::Error, path::PathBuf};
use std::io;

use events::EXIT_KEY;

// use comm::FuzzingEvent;
use termion::input::{MouseTerminal};
use termion::raw::IntoRawMode;
use termion::screen::AlternateScreen;

use tui::{
    backend::TermionBackend,
    Terminal,
};

use self::framework::UserInput;

pub fn launch_app(root_path: PathBuf) -> Result<(), Box<dyn Error>> {
    
    // Terminal initialization
    let stdout = io::stdout().into_raw_mode()?;
    let stdout = MouseTerminal::from(stdout);
    let stdout = AlternateScreen::from(stdout);
    let backend = TermionBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let events = events::Events::<()>::new();

    let mut state = app::State::new(root_path);

    'main_loop: loop {
        terminal.draw(|f| {
            state.draw(f);
        })?;

        // blocking
        let event = events.next()?;

        // first check if the event needs to be intercepted, 
        // for example if it's the exit key
        match &event {
            events::Event::UserInput(UserInput::Key(key)) => {
                if key == &EXIT_KEY {
                    break 'main_loop
                }
            }
            events::Event::_Subscription(_) => {
                
            }
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