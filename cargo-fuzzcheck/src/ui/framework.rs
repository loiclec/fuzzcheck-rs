use termion::event::Key;
use tui::{
    backend::Backend,
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders},
    Frame,
};

pub trait ViewState {
    type Update;
    type InMessage;
    type OutMessage;

    fn convert_in_message(&self, message: Self::InMessage) -> Option<Self::Update>;
    fn update(&mut self, u: Self::Update) -> Option<Self::OutMessage>;
    fn draw<B>(&self, frame: &mut Frame<B>, theme: &Theme, chunk: Rect)
    where
        B: Backend;
}

pub trait ParentView<Child: ViewState>: ViewState {
    fn convert_child_update(update: Child::Update) -> Self::Update;

    fn convert_to_child_in_message(message: Self::InMessage) -> Option<Child::InMessage>;

    fn handle_child_in_message(child: &Child, message: Self::InMessage) -> Option<Self::Update> {
        Self::convert_to_child_in_message(message)
            .and_then(|message| child.convert_in_message(message))
            .map(|child_update| Self::convert_child_update(child_update))
    }

    fn convert_child_out_message(&self, message: Child::OutMessage) -> Either<Self::Update, Self::OutMessage>;

    fn handle_child_out_message(&mut self, message: Child::OutMessage) -> Option<Self::OutMessage> {
        match self.convert_child_out_message(message) {
            Either::Left(u) => self.update(u),
            Either::Right(out) => Some(out),
        }
    }
}

pub trait Focusable {
    fn focus(&mut self);
    fn unfocus(&mut self);
}

pub trait InnerFocusable {
    type Focus: Copy;

    fn focus(&mut self) -> &mut Self::Focus;

    fn view_in_focus(&mut self) -> Option<&mut dyn Focusable>;

    fn update_focus(&mut self, focus: Self::Focus) {
        if let Some(v) = self.view_in_focus() {
            v.unfocus()
        };
        *self.focus() = focus;
        if let Some(v) = self.view_in_focus() {
            v.focus()
        };
    }
}

pub enum Either<A, B> {
    Left(A),
    Right(B),
}

// pub enum Move {
//     Vertical(VerticalMove),
//     Horizontal(HorizontalMove),
// }
pub enum VerticalMove {
    Up,
    Down,
}
pub enum HorizontalMove {
    Left,
    Right,
}

impl VerticalMove {
    pub fn from(input: &Key) -> Option<Self> {
        match input {
            Key::Up => Some(Self::Up),
            Key::Down => Some(Self::Down),
            _ => None,
        }
    }
}
impl HorizontalMove {
    pub fn from(input: &Key) -> Option<Self> {
        match input {
            Key::Left => Some(Self::Left),
            Key::Right => Some(Self::Right),
            _ => None,
        }
    }
}
// impl Move {
//     pub fn from(input: &UserInput) -> Option<Self> {
//         HorizontalMove::from(input).map(Move::Horizontal)
//         .or(VerticalMove::from(input).map(Move::Vertical))
//     }
// }

pub struct Theme {
    pub block_borders: Borders,
    pub default: Style,
    pub highlight: Style,
    pub block_highlight: Style,
}

impl Theme {
    pub fn primary() -> Self {
        Self {
            block_borders: Borders::ALL,
            default: Style::default().bg(Color::Black).fg(Color::White),
            highlight: Style::default().bg(Color::Yellow).fg(Color::Black),
            block_highlight: Style::default().bg(Color::Black).fg(Color::Yellow),
        }
    }
    pub fn secondary() -> Self {
        Self {
            block_borders: Borders::ALL,
            default: Style::default().bg(Color::Black).fg(Color::DarkGray),
            highlight: Style::default().bg(Color::DarkGray).fg(Color::Gray),
            block_highlight: Style::default().bg(Color::Black).fg(Color::White),
        }
    }
    pub fn style_block_default<'a>(&self, block: Block<'a>) -> Block<'a> {
        block.borders(self.block_borders).style(self.default)
    }
    pub fn style_block_highlight<'a>(&self, block: Block<'a>) -> Block<'a> {
        block.borders(self.block_borders).style(self.block_highlight)
    }
}
