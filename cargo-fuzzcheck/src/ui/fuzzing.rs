use std::{cmp::Ordering, collections::{HashMap, HashSet}};

use fuzzcheck_common::ipc::{FuzzerEvent, FuzzerStats, TuiMessage, TuiMessageEvent};
use termion::event::Key;

use super::framework::{Focusable, InnerFocusable, Theme, ViewState};

use tui::{
    backend::Backend,
    layout::{Alignment, Constraint, Corner, Direction, Layout, Rect},
    style::{Color, Style},
    symbols,
    text::{Span, Spans, Text},
    widgets::{Axis, Block, Borders, Chart, Dataset, GraphType, List, ListItem, ListState, Paragraph, Tabs, Wrap},
    Frame,
};

pub struct FuzzingView {
    corpus: HashMap<String, String>,
    events: Vec<TuiMessageEvent>,
    focus: Focus,
    shown_chart: ShownChart
}

#[derive(Clone, Copy)]
pub enum ShownChart {
    Score,
    AvgComplexity,
    ExecPerS,
}
impl ShownChart {
    fn next(self) -> ShownChart {
        match self {
            ShownChart::Score => { ShownChart::AvgComplexity }
            ShownChart::AvgComplexity => { ShownChart::ExecPerS }
            ShownChart::ExecPerS => { ShownChart::Score }
        }
    }
    fn label(self) -> &'static str {
        match self {
            ShownChart::Score => { "score" }
            ShownChart::AvgComplexity => { "avg input cplx" }
            ShownChart::ExecPerS => { "exec/s" }
        }
    }
}

pub enum InMessage {
    Key(Key),
    TuiMessage(TuiMessage)
}

#[derive(Clone, Copy)]
pub enum Focus {
    Chart,
}

impl FuzzingView {
    pub fn new() -> Self {
        Self {
            corpus: HashMap::new(),
            events: vec![],
            focus: Focus::Chart,
            shown_chart: ShownChart::Score,
        }
    }
}

fn event_to_string(event: &FuzzerEvent) -> String {
    match event {
        FuzzerEvent::Start => "Start",
        FuzzerEvent::End => "End",
        FuzzerEvent::CrashNoInput => "Fuzzcheck crashed, but the crashing input could not be retrieved",
        FuzzerEvent::Done => "Done",
        FuzzerEvent::New => "NEW",
        FuzzerEvent::Replace(x) => "RPLC",
        FuzzerEvent::ReplaceLowestStack(_) => "STCK",
        FuzzerEvent::Remove => "RMV ",
        FuzzerEvent::DidReadCorpus => "did read corpus",
        FuzzerEvent::CaughtSignal(_) => "Signal Caught",
        FuzzerEvent::TestFailure => "Test Failure",
    }
    .to_string()
}

pub enum Update {
    SwitchChart,
    AddMessage(TuiMessage),
}
pub enum OutMessage {}

impl ViewState for FuzzingView {
    type Update = self::Update;
    type InMessage = self::InMessage;
    type OutMessage = self::OutMessage;

    fn convert_in_message(&self, message: Self::InMessage) -> Option<Update> {
        match message {
            InMessage::Key(Key::Char('s')) => {
                Some(Update::SwitchChart)
            }
            InMessage::TuiMessage(message) => {
                Some(Update::AddMessage(message))
            }
            _ => {
                None
            }
        }
    }

    fn update(&mut self, u: Update) -> Option<OutMessage> {
        match u {
            Update::AddMessage(x) => {
                match x {
                    TuiMessage::AddInput { hash, input } => {
                        self.corpus.insert(hash, input);
                    }
                    TuiMessage::RemoveInput { hash, input } => {
                        self.corpus.remove(&hash);
                    }
                    TuiMessage::ReportEvent(e) => {
                        self.events.push(e);
                    }
                }
                None
            }
            Update::SwitchChart => {
                self.shown_chart = self.shown_chart.next();
                None
            }
        }
    }

    fn draw<B>(&self, frame: &mut Frame<B>, theme: &Theme, area: Rect)
    where
        B: Backend,
    {
        let block = Block::default().style(theme.default);
        frame.render_widget(block, area);

        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(15), Constraint::Min(0)].as_ref())
            .split(area);

        let mut list_state = ListState::default();
        list_state.select(if self.events.is_empty() { None } else { Some(0) });

        let event_list = List::new(
            self.events
                .iter()
                .rev()
                .map(|x| x.event)
                .map(|x| event_to_string(&x))
                .map(|s| ListItem::new(s).style(theme.default))
                .collect::<Vec<_>>(),
        )
        .block(
            Block::default()
                .title("Events")
                .borders(Borders::ALL)
                .style(theme.default),
        )
        .style(theme.default)
        .highlight_style(theme.emphasis)
        .start_corner(Corner::TopLeft);

        frame.render_stateful_widget(event_list, chunks[0], &mut list_state);

        let right_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)].as_ref())
            .split(chunks[1]);

        let data = self
            .events
            .iter()
            .map(|e| (e.time_ms as f64, 
                match self.shown_chart {
                    ShownChart::Score => { e.stats.score }
                    ShownChart::AvgComplexity => { e.stats.avg_cplx }
                    ShownChart::ExecPerS => { e.stats.exec_per_s as f64 }
                } )
            )
            .collect::<Box<[_]>>();

        let datasets = vec![Dataset::default()
            .marker(symbols::Marker::Dot)
            .graph_type(GraphType::Line)
            .style(theme.emphasis)
            .data(&data)];

        let max_data = data.iter().map(|x| x.1).max_by(|x, y| x.partial_cmp(y).unwrap_or(Ordering::Equal)).unwrap_or(0.0);
        let max_data_str = format!("{:.1}", max_data);
        let half_max_data_str = format!("{:.1}", max_data / 2.0);

        let last_time = self.events.last().map(|x| x.time_ms).unwrap_or(0);
        let last_time_str = format!("{:.2} seconds", last_time as f64 / 1000.0);

        let focused_chart = matches!(self.focus, Focus::Chart);

        let chart_block_style = if focused_chart {
            theme.block_highlight
        } else {
            theme.default
        };

        let chart = Chart::new(datasets)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .style(chart_block_style)
                    .title(vec![
                        Span::styled(self.shown_chart.label(), chart_block_style),
                        Span::styled("... press ", chart_block_style),
                        Span::styled("s", if focused_chart {
                            theme.highlight
                        } else {
                            theme.emphasis
                        }),
                        Span::styled(" to show next chart", chart_block_style),
                    ]),
            )
            .x_axis(
                Axis::default()
                    .title(Span::styled("time", theme.emphasis))
                    .style(theme.emphasis)
                    .bounds([0.0, last_time as f64])
                    .labels(
                        ["0", &last_time_str]
                            .iter()
                            .cloned()
                            .map(|label| Span::styled(label, theme.emphasis))
                            .collect(),
                    )
            )
            .y_axis(
                Axis::default()
                    .title(Span::styled(self.shown_chart.label(), theme.emphasis))
                    .style(theme.emphasis)
                    .bounds([0.0, max_data])
                    .labels(
                        ["0.0", &half_max_data_str, &max_data_str]
                            .iter()
                            .cloned()
                            .map(|label| Span::styled(label, theme.emphasis))
                            .collect(),
                    ),
            )
            .style(theme.default);

        frame.render_widget(chart, right_chunks[0]);
    }
}

impl InnerFocusable for FuzzingView {
    type Focus = self::Focus;

    fn focus(&mut self) -> &mut Self::Focus {
        &mut self.focus
    }

    fn view_in_focus(&mut self) -> Option<&mut dyn super::framework::Focusable> {
        match self.focus {
            Focus::Chart => None,
        }
    }
}
