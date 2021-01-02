use framework::{HorizontalMove, ViewState};
use hlist::HList;
use tui::{
    layout::Rect,
    widgets::{Block, Borders},
};

use super::{
    framework::{self, Focusable, Theme},
    hlist::{self, HListItem, HListState},
};

pub struct HorizontalListView {
    pub items: Vec<String>,
    pub state: HListState,
    pub title: String,
    pub focused: bool,
}

impl HorizontalListView {
    pub fn new(title: &str, items: impl Iterator<Item = String>) -> Self {
        let items = items.collect::<Vec<_>>();
        let mut state = HListState::default();
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

// pub enum Update {
//     Select(usize)
// }

// pub enum InMessage {
//     HorizontalMove(HorizontalMove),
//     Tab
// }

pub enum OutMessage {
    Select(usize),
}

impl Focusable for HorizontalListView {
    fn focus(&mut self) {
        self.focused = true;
    }

    fn unfocus(&mut self) {
        self.focused = false;
    }
}

impl ViewState for HorizontalListView {
    type Update = HorizontalMove;
    type InMessage = HorizontalMove;

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
                HorizontalMove::Left => {
                    if selected > 0 {
                        self.state.select(Some(selected - 1));
                        Some(Self::OutMessage::Select(selected - 1))
                    } else {
                        None
                    }
                }
                HorizontalMove::Right => {
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
        let list_items = self.items.iter().map(|s| HListItem::new(s.clone())).collect::<Vec<_>>();
        let list = HList::new(list_items)
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
