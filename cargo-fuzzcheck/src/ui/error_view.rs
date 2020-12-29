


use std::fmt::Debug;
use tui::{Frame, backend::Backend, layout::{Alignment, Constraint, Direction, Layout}, style::{Color, Style}, text::Text, widgets::{Block, Borders, Paragraph, Wrap}};
use super::framework::{UserInput, default_style, highlight_style};

pub struct State {
    error: Box<dyn Debug>,
}

impl State {
    pub fn new(error: Box<dyn Debug>) -> Self {
        Self {
            error
        }
    }
}

pub struct Update;
pub struct OutMessage;

impl State {
    pub fn convert_in_message(&self, _input: UserInput) -> Option<Update> {
        Some(Update)
    }

    pub fn update(&mut self, _u: Update) -> Option<OutMessage> {
        Some(OutMessage)
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
            .constraints([Constraint::Length(10), Constraint::Length(3), Constraint::Min(0)].as_ref())
            .split(chunks[1]);

        let text = Text::from(format!(
            r#"Error: {:?}

            Press 'q' or Enter to exit."#
        , self.error));
        let p = Paragraph::new(text)
            .block(Block::default().borders(Borders::ALL))
            .style(default_style())
            .alignment(Alignment::Left)
            .wrap(Wrap { trim: true });

        frame.render_widget(p, bottom_chunks[0]);


        let quit_button = Paragraph::new(Text::raw("Quit")).block(Block::default().borders(Borders::ALL)).alignment(Alignment::Center).style(highlight_style());

        frame.render_widget(quit_button, bottom_chunks[1]);
    }
}
