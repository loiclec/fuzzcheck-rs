use std::iter::Iterator;
use tui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    text::{Span, Spans},
    widgets::{Block, StatefulWidget, Widget},
};

#[derive(Debug, Clone)]
pub struct HListState {
    offset: usize,
    selected: Option<usize>,
}

impl Default for HListState {
    fn default() -> HListState {
        HListState {
            offset: 0,
            selected: None,
        }
    }
}

impl HListState {
    pub fn selected(&self) -> Option<usize> {
        self.selected
    }

    pub fn select(&mut self, index: Option<usize>) {
        self.selected = index;
        if index.is_none() {
            self.offset = 0;
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct HListItem<'a> {
    content: Spans<'a>,
    style: Style,
}

impl<'a> HListItem<'a> {
    pub fn new<T>(content: T) -> HListItem<'a>
    where
        T: Into<Spans<'a>>,
    {
        HListItem {
            content: content.into(),
            style: Style::default(),
        }
    }

    pub fn style(mut self, style: Style) -> HListItem<'a> {
        self.style = style;
        self
    }

    pub fn width(&self) -> usize {
        self.content.width() + 2
    }
}

/// A widget to display several items among which one can be selected (optional)
///
/// # Examples
///
/// ```
/// # use tui::widgets::{Block, Borders, List, ListItem};
/// # use tui::style::{Style, Color, Modifier};
/// let items = [ListItem::new("Item 1"), ListItem::new("Item 2"), ListItem::new("Item 3")];
/// List::new(items)
///     .block(Block::default().title("List").borders(Borders::ALL))
///     .style(Style::default().fg(Color::White))
///     .highlight_style(Style::default().add_modifier(Modifier::ITALIC))
/// ```
#[derive(Debug, Clone)]
pub struct HList<'a> {
    block: Option<Block<'a>>,
    items: Vec<HListItem<'a>>,
    /// Style used as a base style for the widget
    style: Style,
    /// Style used to render selected item
    highlight_style: Style,
}

impl<'a> HList<'a> {
    pub fn new<T>(items: T) -> HList<'a>
    where
        T: Into<Vec<HListItem<'a>>>,
    {
        HList {
            block: None,
            style: Style::default(),
            items: items.into(),
            highlight_style: Style::default(),
        }
    }

    pub fn block(mut self, block: Block<'a>) -> HList<'a> {
        self.block = Some(block);
        self
    }

    pub fn style(mut self, style: Style) -> HList<'a> {
        self.style = style;
        self
    }

    pub fn highlight_style(mut self, style: Style) -> HList<'a> {
        self.highlight_style = style;
        self
    }
}

impl<'a> StatefulWidget for HList<'a> {
    type State = HListState;

    fn render(mut self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        buf.set_style(area, self.style);
        let list_area = match self.block.take() {
            Some(b) => {
                let inner_area = b.inner(area);
                b.render(area, buf);
                inner_area
            }
            None => area,
        };

        if list_area.width < 1 || list_area.height < 1 {
            return;
        }

        if self.items.is_empty() {
            return;
        }
        let list_width = list_area.width as usize;

        let mut start = state.offset;
        let mut end = state.offset;
        let mut width = 0;

        for item in self.items.iter().skip(state.offset) {
            if width + item.width() > list_width {
                break;
            }
            width += item.width();
            end += 1;
        }

        let selected = state.selected.unwrap_or(0).min(self.items.len() - 1);
        while selected >= end {
            width = width.saturating_add(self.items[end].width());
            end += 1;
            while width > list_width {
                width = width.saturating_sub(self.items[start].width());
                start += 1;
            }
        }
        while selected < start {
            start -= 1;
            width = width.saturating_add(self.items[start].width());
            while width > list_width {
                end -= 1;
                width = width.saturating_sub(self.items[end].width());
            }
        }
        state.offset = start;

        let mut current_width = 0;

        for (i, item) in self.items.iter_mut().enumerate().skip(state.offset).take(end - start) {
            let (x, y) = {
                let pos = (list_area.left() + current_width, list_area.top());
                pos
            };

            let area = Rect {
                x,
                y,
                width: item.width() as u16,
                height: list_area.height,
            };
            let item_style = self.style.patch(item.style);
            buf.set_style(area, item_style);

            let is_selected = state.selected.map(|s| s == i).unwrap_or(false);

            let max_element_width = list_area.width - current_width;

            buf.set_span(x, y, &Span::styled(" ", item.style), 1);
            buf.set_spans(x + 1, y as u16, &item.content, max_element_width);

            if is_selected {
                buf.set_style(area, self.highlight_style);
            }
            current_width += item.width() as u16;
        }
    }
}

impl<'a> Widget for HList<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let mut state = HListState::default();
        StatefulWidget::render(self, area, buf, &mut state);
    }
}
