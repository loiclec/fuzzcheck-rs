use std::{cmp::Ordering, collections::{HashMap}, iter};

use fuzzcheck_common::ipc::{FuzzerEvent, TuiMessage, TuiMessageEvent};
use termion::event::Key;

use super::{framework::{AnyView, Either, InnerFocusable, ParentView, SwitchFocus, Theme, ViewState}, vertical_list_view::{self, VerticalListView}};

use tui::{Frame, backend::Backend, layout::{Alignment, Constraint, Direction, Layout, Rect}, style::{Color, Style}, symbols, text::{Span}, widgets::{Axis, Block, Borders, Chart, Dataset, GraphType, Paragraph, Wrap}};

enum Status {
    Running,
    Paused,
    Stopped
}
impl std::fmt::Display for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", match self {
            Status::Running => { "Running" }
            Status::Paused => { "Paused" }
            Status::Stopped => { "Stopped" }
        })
    }
}

pub struct FuzzingView {
    corpus: HashMap<String, String>,
    events: Vec<TuiMessageEvent>,
    artifact: Option<(String, String)>,

    status: Status,
    focus: Focus,

    list_view: VerticalListView,

    shown_chart: ShownChart,
    detail_view: (String, String),
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
            ShownChart::Score => ShownChart::AvgComplexity,
            ShownChart::AvgComplexity => ShownChart::ExecPerS,
            ShownChart::ExecPerS => ShownChart::Score,
        }
    }
    fn label(self) -> &'static str {
        match self {
            ShownChart::Score => "Score",
            ShownChart::AvgComplexity => "Average input complexity",
            ShownChart::ExecPerS => "Iterations per second",
        }
    }
}

pub enum InMessage {
    Key(Key),
    TuiMessage(TuiMessage),
}

#[derive(Clone, Copy)]
pub struct Focus {
    x: u8,
    y: u8,
}
impl Focus {
    fn focused_part(self) -> FocusedPart {
        match (self.x, self.y) {
            (0 , _) => FocusedPart::Events,
            (1, 0) => FocusedPart::Chart,
            (1, _) => FocusedPart::Details,
            _ => FocusedPart::Chart,
        }
    }
}

#[derive(Clone, Copy)]
pub enum FocusedPart {
    Events,
    Chart,
    Details,
}
impl FocusedPart {
    fn canonical_position(self) -> Focus {
        match self {
            FocusedPart::Events => { Focus { x: 0, y: 0 } }
            FocusedPart::Chart => { Focus { x: 1, y: 0 } }
            FocusedPart::Details => { Focus { x: 1, y: 1 } }
        }
    }
    fn next(self) -> Self {
        match self {
            FocusedPart::Events => { FocusedPart::Chart }
            FocusedPart::Chart => { FocusedPart::Details }
            FocusedPart::Details => { FocusedPart::Events }
        }
    }
    fn prev(self) -> Self {
        match self {
            FocusedPart::Events => { FocusedPart::Details }
            FocusedPart::Chart => { FocusedPart::Events  }
            FocusedPart::Details => { FocusedPart::Chart }
        }
    }
}

impl FuzzingView {
    pub fn new() -> Self {
        Self {
            corpus: HashMap::new(),
            events: vec![],
            artifact: None,
            status: Status::Running,
            focus: FocusedPart::Chart.canonical_position(),
            list_view: VerticalListView::new("Events", iter::empty()),
            shown_chart: ShownChart::Score,
            detail_view: ("".to_string(), "".to_string()),
        }
    }
}

fn event_to_string(event: &FuzzerEvent) -> String {
    match event {
        FuzzerEvent::Start => "Start".to_string(),
        FuzzerEvent::End => "End".to_string(),
        FuzzerEvent::CrashNoInput => "Fuzzcheck crashed, but the crashing input could not be retrieved".to_string(),
        FuzzerEvent::Done => "Done".to_string(),
        FuzzerEvent::New => "NEW".to_string(),
        FuzzerEvent::Replace(x) => format!("RPLC {}", x),
        FuzzerEvent::ReplaceLowestStack(_) => "STCK".to_string(),
        FuzzerEvent::Remove => "RMV ".to_string(),
        FuzzerEvent::DidReadCorpus => "did read corpus".to_string(),
        FuzzerEvent::CaughtSignal(_) => "Signal Caught".to_string(),
        FuzzerEvent::TestFailure => "Test Failure".to_string(),
    }
}

pub enum Update {
    SwitchChart,
    AddMessage(TuiMessage),
    SwitchFocus(SwitchFocus),
    ListView(vertical_list_view::Update),
    SelectListItem(usize),
    Pause,
    UnPause,
    UnPauseUntilNextEvent,
    Stop,
}
pub enum OutMessage {
    PauseFuzzer,
    UnPauseFuzzer,
    UnPauseFuzzerUntilNextEvent,
    StopFuzzer
}

impl FuzzingView {
    fn update_detail_view_with_event(&mut self, event_idx: usize) {
        let event = &self.events[event_idx];
        self.detail_view = (
            format!("Event #{}", event_idx),
            format!(r#"{}
            score: {:.2} average input cplx: {:.2}"#, 
            event_to_string(&event.event),
            event.stats.score, event.stats.avg_cplx)
        );
    }
}

impl AnyView for FuzzingView {
    fn focus(&mut self) {
    }

    fn unfocus(&mut self) {
    }

    fn key_bindings(&self) -> Vec<(Key, String)> {
        let mut map = Vec::new();
        if !matches!(self.status, Status::Stopped) {
            map.push((Key::Char('x'), "stop fuzzing".to_string()));
        }
        match self.status {
            Status::Running => {
                map.push((Key::Char('p'), "pause fuzzer".to_string()));
            }
            Status::Paused => {
                map.push((Key::Char('u'), "unpause".to_string()));
                map.push((Key::Char('v'), "unpause until next event".to_string()));
            }
            Status::Stopped => {}
        }
        match self.focus.focused_part() {
            FocusedPart::Events => { }
            FocusedPart::Chart => { 
                map.push((Key::Char('s'), "show next chart".to_string()));
            }
            FocusedPart::Details => { }
        }
        map
    }
}

impl ViewState for FuzzingView {
    type Update = self::Update;
    type InMessage = self::InMessage;
    type OutMessage = self::OutMessage;

    fn convert_in_message(&self, message: Self::InMessage) -> Option<Update> {
        match self.focus.focused_part() {
            FocusedPart::Events => {
                if let Some(u) = Self::handle_child_in_message(&self.list_view, &message) {
                    return Some(u);
                }
            }
            _ => {}
        }
        match message {
            InMessage::Key(k) => {
                if let Some(sf) = SwitchFocus::from_standard_key(&k) {
                    return Some(Update::SwitchFocus(sf));
                }
                match k {
                    Key::Char('s') => {
                        Some(Update::SwitchChart)
                    }
                    Key::Char('p') => {
                        Some(Update::Pause)
                    }
                    Key::Char('u') => {
                        Some(Update::UnPause)
                    }
                    Key::Char('x') => {
                        Some(Update::Stop)
                    }
                    Key::Char('v') => {
                        Some(Update::UnPauseUntilNextEvent)
                    }
                    _ => {
                        None
                    }
                }
            }
            InMessage::TuiMessage(message) => Some(Update::AddMessage(message)),
        }
    }

    fn update(&mut self, u: Update) -> Option<OutMessage> {
        match u {
            Update::AddMessage(x) => {
                match x {
                    TuiMessage::AddInput { hash, input } => {
                        self.corpus.insert(hash, input);
                    }
                    TuiMessage::RemoveInput { hash, input: _ } => {
                        self.corpus.remove(&hash);
                    }
                    TuiMessage::ReportEvent(e) => {
                        self.list_view.items.insert(0, event_to_string(&e.event));
                        self.events.push(e);
                        self.list_view.state.select(self.list_view.state.selected().map(|x| x + 1));
                        if !self.list_view.focused {
                            self.list_view.state.select(None);
                            self.update_detail_view_with_event(self.events.len() - 1);
                        }
                    }
                    TuiMessage::SaveArtifact { hash, input } => {
                        self.status = Status::Stopped;
                        self.artifact = Some((hash.clone(), input.clone()));
                        self.detail_view = (format!("Artifact: {}", hash), input);
                    }
                    TuiMessage::Paused => {
                        self.status = Status::Paused;
                    }
                    TuiMessage::UnPaused => {
                        self.status = Status::Running;
                    }
                    TuiMessage::Stopped => {
                        self.status = Status::Stopped;
                    }
                }
                None
            }
            Update::SwitchChart => {
                self.shown_chart = self.shown_chart.next();
                None
            }
            Update::SwitchFocus(sf) => {
                if let Some(f) = self.focus_after_switch(sf) {
                    self.update_focus(f);
                }
                None
            }
            Update::ListView(u) => {
                self.list_view.update(u).and_then(|m| self.handle_child_out_message(m))
            }
            Update::SelectListItem(idx) => {
                let event_idx = self.events.len() - idx - 1;
                self.update_detail_view_with_event(event_idx);
                None
            }
            Update::Pause => {
                Some(OutMessage::PauseFuzzer)
            }
            Update::UnPause => {
                Some(OutMessage::UnPauseFuzzer)
            }
            Update::Stop => {
                Some(OutMessage::StopFuzzer)
            }
            Update::UnPauseUntilNextEvent => {
                Some(OutMessage::UnPauseFuzzerUntilNextEvent)
            }
        }
    }

    fn draw<B>(&self, frame: &mut Frame<B>, theme: &Theme, area: Rect)
    where
        B: Backend,
    {
        let block = Block::default().style(theme.default);
        frame.render_widget(block, area);

        let vertical_chunks = Layout::default().direction(Direction::Vertical).constraints([Constraint::Length(1), Constraint::Min(0)].as_ref()).split(area);

        let status_view = Paragraph::new(format!("{}", self.status)).style(Style::default().bg(Color::White).fg(Color::Black)).alignment(Alignment::Center);
        frame.render_widget(status_view, vertical_chunks[0]);

        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(15), Constraint::Min(0)].as_ref())
            .split(vertical_chunks[1]);

        self.list_view.draw(frame, theme, chunks[0]);

        let right_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)].as_ref())
            .split(chunks[1]);

        let data = self
            .events
            .iter()
            .map(|e| {
                (
                    e.time_ms as f64,
                    match self.shown_chart {
                        ShownChart::Score => e.stats.score,
                        ShownChart::AvgComplexity => e.stats.avg_cplx,
                        ShownChart::ExecPerS => e.stats.exec_per_s as f64,
                    },
                )
            })
            .collect::<Box<[_]>>();

        let datasets = vec![Dataset::default()
            .marker(symbols::Marker::Dot)
            .graph_type(GraphType::Line)
            .style(theme.emphasis)
            .data(&data)];

        let max_data = data
            .iter()
            .map(|x| x.1)
            .max_by(|x, y| x.partial_cmp(y).unwrap_or(Ordering::Equal))
            .unwrap_or(0.0);
        let max_data_str = format!("{:.1}", max_data);
        let half_max_data_str = format!("{:.1}", max_data / 2.0);

        let last_time = self.events.last().map(|x| x.time_ms).unwrap_or(0);
        let last_time_str = format!("{:.2} seconds", last_time as f64 / 1000.0);

        let focused_chart = matches!(self.focus.focused_part(), FocusedPart::Chart);

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
                    ),
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

        let is_detail_view_focused = matches!(self.focus.focused_part(), FocusedPart::Details);

        let detail_view = Paragraph::new(self.detail_view.1.clone())
            .block(Block::default()
                .style(if is_detail_view_focused {
                        theme.block_highlight
                    } else {
                        theme.default
                    }
                ).title(self.detail_view.0.clone())
                .borders(Borders::ALL)
            ).style(theme.default)
            .wrap(Wrap { trim: true });
        frame.render_widget(detail_view, right_chunks[1]);
    
    }
}

impl InnerFocusable for FuzzingView {
    type Focus = self::Focus;

    fn focus(&mut self) -> &mut Self::Focus {
        &mut self.focus
    }

    fn focus_after_switch(&self, sf: SwitchFocus) -> Option<Self::Focus> {
        let mut copy = self.focus;
        match sf {
            SwitchFocus::Up => {
                copy.y = copy.y.saturating_sub(1);
            }
            SwitchFocus::Right => {
                copy.x = std::cmp::min(copy.x + 1, 1);
            }
            SwitchFocus::Down => {
                copy.y = std::cmp::min(copy.y + 1, 1);
            }
            SwitchFocus::Left => {
                copy.x = copy.x.saturating_sub(1);
            }
            SwitchFocus::Next => {
                copy = self.focus.focused_part().next().canonical_position();
            }
            SwitchFocus::Previous => {
                copy = self.focus.focused_part().prev().canonical_position();
            }
            SwitchFocus::In => {
                return None
            }
            SwitchFocus::Out => {
                return None
            }
        }
        Some(copy)
    }

    fn view_in_focus_ref(&self) -> Option<&dyn AnyView> {
        match self.focus.focused_part() {
            FocusedPart::Events => { Some(&self.list_view) }
            FocusedPart::Chart => { None }
            FocusedPart::Details => { None }
        }
    }

    fn view_in_focus_mut(&mut self) -> Option<&mut dyn AnyView> {
        match self.focus.focused_part() {
            FocusedPart::Events => { Some(&mut self.list_view) }
            FocusedPart::Chart => { None }
            FocusedPart::Details => { None }
        }
    }
}

impl ParentView<VerticalListView> for FuzzingView {
    fn convert_child_update(update: vertical_list_view::Update) -> Self::Update {
        Update::ListView(update)
    }

    fn convert_to_child_in_message(message: &Self::InMessage) -> Option<vertical_list_view::InMessage> {
        match message {
            InMessage::Key(k) => {
                vertical_list_view::InMessage::from(k)
            }
            InMessage::TuiMessage(_) => {
                None
            }
        }
    }

    fn convert_child_out_message(&self, message: vertical_list_view::OutMessage) -> Either<Self::Update, Self::OutMessage> {
        match message {
            vertical_list_view::OutMessage::Select(idx) => {
                Either::Left(Update::SelectListItem(idx))
            }
        }
    }
}
