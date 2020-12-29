

use std::{path::PathBuf};

use termion::event::Key;
use tui::{Frame, backend::Backend, layout::{Alignment, Constraint, Direction, Layout}, style::{Color, Style}, text::{Span, Spans, Text}, widgets::{Block, Borders, Paragraph, Wrap}};

use crate::{CargoFuzzcheckError, project::{NonInitializedRoot, Root, read::NonInitializedRootError}};

use super::framework::{Move, UserInput, default_style, highlight_style};

pub struct State {
    pub root: Root,
    focus: Focus,
}

impl State {
    pub fn new(root: Root) -> Self {
        Self {
            root,
            focus: Focus::Quit
        }
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
        None
    }

    pub fn draw<B>(&mut self, frame: &mut Frame<B>) where B: Backend {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(5), Constraint::Min(0)].as_ref())
            .split(frame.size());

        let block = Block::default()
            .style(Style::default().bg(Color::Black));

        frame.render_widget(block, frame.size());

        let fuzz_targets = self.root.fuzz.non_instrumented.fuzz_targets.targets.keys().map(|k| Span::from(k.to_str().unwrap()) ).collect::<Vec<_>>();
        let spans = Spans::from(fuzz_targets);
        let list = Paragraph::new(spans);

        frame.render_widget(list, chunks[1])
    }
}
