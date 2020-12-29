use termion::event::Key;
use tui::{Frame, backend::Backend, style::{Color, Style}};

pub enum Either<A,B> {
    Left(A),
    Right(B)
}

pub enum UserInput {
    Key(Key),
}

pub enum Move {
    Up,
    Down,
    Left,
    Right
}
impl Move {
    pub fn from(input: &UserInput) -> Option<Self> {
        match input {
            UserInput::Key(Key::Up) => {
                Some(Self::Up)
            }
            UserInput::Key(Key::Down) => {
                Some(Self::Down)
            }
            UserInput::Key(Key::Left) => {
                Some(Self::Left)
            }
            UserInput::Key(Key::Right) => {
                Some(Self::Right)
            }
            _ => None
        }
    }
}

pub fn default_style() -> Style {
    Style::default().bg(Color::Red).fg(Color::White)
}
pub fn highlight_style() -> Style {
    default_style().bg(Color::Red).fg(Color::Yellow)
}

trait App {
    type Update;
    type InMessage;
    type OutMessage;

    fn convert_in_message(&self, message: Self::InMessage) -> Option<Self::Update>;
    fn update(&mut self, u: Self::Update) -> Option<Self::OutMessage>;
    fn draw<B>(&mut self, frame: &mut Frame<B>) where B: Backend;
}
