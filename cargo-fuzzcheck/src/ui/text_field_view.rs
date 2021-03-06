use std::cmp::min;

use framework::{AnyView, HorizontalMove};

use termion::event::Key;
use tui::{layout::Rect, widgets::Paragraph};

use super::framework::{self, Theme};

pub struct TextFieldView {
    input: String,
    caret: usize,
    focused: bool,
}

impl TextFieldView {
    pub fn new(input: String) -> Self {
        let input_len = input.len();
        Self {
            input,
            caret: input_len,
            focused: false,
        }
    }
}

pub enum Update {
    Insert(char),
    Delete,
    MoveCaret(HorizontalMove),
}

pub enum OutMessage {
    Edited(String),
}

impl AnyView for TextFieldView {
    fn focus(&mut self) {
        self.focused = true;
    }

    fn unfocus(&mut self) {
        self.focused = false;
    }
    fn key_bindings(&self) -> Vec<(Key, String)> {
        Vec::new()
    }
}

impl framework::ViewState for TextFieldView {
    type Update = self::Update;
    type InMessage = Key;

    type OutMessage = self::OutMessage;

    fn convert_in_message(&self, message: Self::InMessage) -> Option<Self::Update> {
        match message {
            Key::Left => Some(Update::MoveCaret(HorizontalMove::Left)),
            Key::Right => Some(Update::MoveCaret(HorizontalMove::Right)),
            Key::Backspace => Some(Update::Delete),
            Key::Char('\t') => None, // disallow tabs
            Key::Char('\n') => None, // disallow newlines
            Key::Char(c) => Some(Update::Insert(c)),
            _ => None,
        }
    }

    fn update(&mut self, u: Self::Update) -> Option<Self::OutMessage> {
        match u {
            Update::Insert(c) => {
                self.input.insert(self.caret, c);
                self.caret = self.caret + 1;
                Some(OutMessage::Edited(self.input.clone()))
            }
            Update::Delete => {
                if self.caret > 0 {
                    self.input.remove(self.caret - 1);
                    self.caret = self.caret.saturating_sub(1);
                    Some(OutMessage::Edited(self.input.clone()))
                } else {
                    None
                }
            }
            Update::MoveCaret(HorizontalMove::Left) => {
                self.caret = self.caret.saturating_sub(1);
                None
            }
            Update::MoveCaret(HorizontalMove::Right) => {
                self.caret = min(self.input.len(), self.caret + 1);
                None
            }
        }
    }

    fn draw<B>(&self, frame: &mut tui::Frame<B>, theme: &Theme, chunk: Rect)
    where
        B: tui::backend::Backend,
    {
        let p = Paragraph::new(self.input.as_str()).style(if self.focused { theme.highlight } else { theme.default });
        frame.render_widget(p, chunk);
        if theme.focus && self.focused {
            frame.set_cursor(chunk.x + self.caret as u16, chunk.y)
        }
    }
}
