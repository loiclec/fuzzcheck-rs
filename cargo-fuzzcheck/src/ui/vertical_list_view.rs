use framework::{VerticalMove, ViewState};

use tui::{
    layout::Rect,
    widgets::{Block, Borders, List, ListItem, ListState},
};

use super::framework::{self, Focusable, Theme};

pub struct VerticalListView {
    pub items: Vec<String>,
    pub state: ListState,
    pub title: String,
    pub focused: bool,
}

impl VerticalListView {
    pub fn new(title: &str, items: impl Iterator<Item = String>) -> Self {
        let items = items.collect::<Vec<_>>();
        let mut state = ListState::default();
        if !items.is_empty() {
            state.select(Some(0));
        }
        Self {
            items,
            state,
            title: title.to_string(),
            focused: false,
        }
    }
}

pub type Update = VerticalMove;
pub type InMessage = VerticalMove;

pub enum OutMessage {
    Select(usize),
}

impl Focusable for VerticalListView {
    fn focus(&mut self) {
        self.focused = true;
    }

    fn unfocus(&mut self) {
        self.focused = false;
    }
}

impl ViewState for VerticalListView {
    type Update = VerticalMove;
    type InMessage = VerticalMove;

    type OutMessage = self::OutMessage;

    fn convert_in_message(&self, message: Self::InMessage) -> Option<Self::Update> {
        Some(message)
    }

    fn update(&mut self, u: Self::Update) -> Option<Self::OutMessage> {
        if self.items.is_empty() {
            return None;
        }
        if let Some(selected) = self.state.selected() {
            match u {
                VerticalMove::Up => {
                    if selected > 0 {
                        self.state.select(Some(selected - 1));
                        Some(Self::OutMessage::Select(selected - 1))
                    } else {
                        None
                    }
                }
                VerticalMove::Down => {
                    if selected < self.items.len() - 1 {
                        self.state.select(Some(selected + 1));
                        Some(Self::OutMessage::Select(selected + 1))
                    } else {
                        None
                    }
                }
            }
        } else {
            self.state.select(Some(0));
            Some(OutMessage::Select(0))
        }
    }

    fn draw<B>(&self, frame: &mut tui::Frame<B>, theme: &Theme, chunk: Rect)
    where
        B: tui::backend::Backend,
    {
        let inner_theme = if self.focused {
            Theme::primary()
        } else {
            Theme::secondary()
        };
        let list_items = self.items.iter().map(|s| ListItem::new(s.clone())).collect::<Vec<_>>();
        let list = List::new(list_items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(self.title.as_str())
                    .style(if self.focused {
                        theme.block_highlight
                    } else {
                        theme.default
                    }),
            )
            .style(inner_theme.default)
            .highlight_style(inner_theme.highlight);

        let mut state = self.state.clone();
        frame.render_stateful_widget(list, chunk, &mut state)
    }
}
