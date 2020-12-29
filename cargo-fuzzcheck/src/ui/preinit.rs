

use std::{path::{Path, PathBuf}};

use termion::event::Key;
use tui::{Frame, backend::Backend, layout::{Alignment, Constraint, Direction, Layout}, style::{Color, Style}, text::Text, widgets::{Block, Borders, Paragraph, Wrap}};

use crate::{CargoFuzzcheckError, project::{NonInitializedRoot, read::NonInitializedRootError}};

use super::framework::{Move, UserInput, default_style, highlight_style};

pub struct State {
    pub root_path: PathBuf,
    pub non_initialized_root: NonInitializedRoot,
    focus: Focus,
}

impl State {
    pub fn new(root_path: &Path) -> Result<Self, NonInitializedRootError> {
        let non_initialized_root = NonInitializedRoot::from_path(root_path)?;
        Ok(Self {
            root_path: root_path.to_path_buf(),
            non_initialized_root,
            focus: Focus::Quit
        })
    }
}

enum Focus {
    Initialize,
    Quit
}

pub enum Update {
    Initialize(Option<String>),
    Move(Move),
    Quit
}

pub enum OutMessage {
    Initialized,
    Error(CargoFuzzcheckError),
    Quit
}

impl State {
    pub fn convert_in_message(&self, input: UserInput) -> Option<Update> {
        if let Some(mv) = Move::from(&input) {
            return Some(Update::Move(mv))
        }
        match input {
            UserInput::Key(Key::Char('\n')) => {
                match self.focus {
                    Focus::Initialize => { Some(Update::Initialize(None)) }
                    Focus::Quit => { Some(Update::Quit) }
                }
            }
            _ => None,
        }
    }

    pub fn update(&mut self, u: Update) -> Option<OutMessage> {
        match u {
            Update::Initialize(fuzzcheck_path) => {
                let fuzzcheck_path = fuzzcheck_path.unwrap_or(env!("CARGO_PKG_VERSION").to_string());
                let result = self.non_initialized_root.init_command(&fuzzcheck_path);
                match result {
                    Ok(_) => {
                        Some(OutMessage::Initialized)
                    }
                    Err(err) => {
                        Some(OutMessage::Error(err))
                    }
                }
            }
            Update::Move(Move::Left) => {
                match self.focus {
                    Focus::Quit => { self.focus = Focus::Initialize; None }
                    _ => { None }
                }
            }
            Update::Move(Move::Right) => {
                match self.focus {
                    Focus::Initialize => { self.focus = Focus::Quit; None }
                    _ => { None }
                }
            }
            Update::Move(Move::Up) | Update::Move(Move::Down) => { None }
            Update::Quit => {
                Some(OutMessage::Quit)
            }
        }
    }

    pub fn draw<B>(&mut self, frame: &mut Frame<B>) where B: Backend {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(5), Constraint::Min(0)].as_ref())
            .split(frame.size());

        let block = Block::default()
            .style(Style::default().bg(Color::Black));

        frame.render_widget(block, frame.size());

        let bottom_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(0)].as_ref())
            .split(chunks[1]);

        let button_areas = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
            .split(bottom_chunks[0]);

        let text = Text::from("The fuzz folder has not been created yet. Would you like to create it?");
        let p = Paragraph::new(text)
            .block(Block::default().borders(Borders::ALL))
            .style(default_style())
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true });

        frame.render_widget(p, chunks[0]);

        let mut initialize_button = Paragraph::new(Text::raw("Create")).block(Block::default().borders(Borders::ALL)).alignment(Alignment::Center).style(default_style());
    
        let mut quit_button = Paragraph::new(Text::raw("Quit")).block(Block::default().borders(Borders::ALL)).alignment(Alignment::Center).style(default_style());

        match self.focus {
            Focus::Initialize => {
                initialize_button = initialize_button.style(highlight_style());
            }
            Focus::Quit => {
                quit_button = quit_button.style(highlight_style());
            }
        }

        frame.render_widget(initialize_button, button_areas[0]) ;
        frame.render_widget(quit_button, button_areas[1]) ;
    }
}
