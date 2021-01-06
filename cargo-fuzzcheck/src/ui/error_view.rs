use super::framework::{AnyView, Theme, ViewState};
use std::{fmt::Debug};
use termion::event::Key;
use tui::{
    backend::Backend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::Text,
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

pub struct ErrorView {
    error: Box<dyn Debug>,
}

impl ErrorView {
    pub fn new(error: Box<dyn Debug>) -> Self {
        Self { error }
    }
}

pub struct Update;
pub struct OutMessage;

impl AnyView for ErrorView {
    fn focus(&mut self) {
    }

    fn unfocus(&mut self) {
    }

    fn key_bindings(&self) -> Vec<(Key, String)> {
        Vec::new()
    }
}

impl ViewState for ErrorView {
    type Update = self::Update;
    type InMessage = Key;
    type OutMessage = self::OutMessage;

    fn convert_in_message(&self, _input: Key) -> Option<Update> {
        Some(Update)
    }

    fn update(&mut self, _u: Update) -> Option<OutMessage> {
        Some(OutMessage)
    }

    fn draw<B>(&self, frame: &mut Frame<B>, theme: &Theme, area: Rect)
    where
        B: Backend,
    {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(5), Constraint::Min(0)].as_ref())
            .split(area);

        let block = Block::default().style(Style::default().bg(Color::Black));

        frame.render_widget(block, area);

        let bottom_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(10), Constraint::Length(3), Constraint::Min(0)].as_ref())
            .split(chunks[1]);

        let text = Text::from(format!(
            r#"Error: {:?}

            Press 'q' or Enter to exit."#,
            self.error
        ));
        let p = Paragraph::new(text)
            .block(Block::default().borders(Borders::ALL))
            .style(theme.default)
            .alignment(Alignment::Left)
            .wrap(Wrap { trim: true });

        frame.render_widget(p, bottom_chunks[0]);

        let quit_button = Paragraph::new(Text::raw("Quit"))
            .block(Block::default().borders(Borders::ALL))
            .alignment(Alignment::Center)
            .style(theme.highlight);

        frame.render_widget(quit_button, bottom_chunks[1]);
    }
}
