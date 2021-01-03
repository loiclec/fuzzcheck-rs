use fuzzcheck_common::ipc::{FuzzerEvent, FuzzerStats, TuiMessage};

use super::framework::{Theme, ViewState};

use tui::{
    backend::Backend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::Text,
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

pub struct FuzzingView {
    message: String,
}

impl FuzzingView {
    pub fn new() -> Self {
        Self {
            message: "initial".to_string(),
        }
    }
}

fn stats_to_string(stats: FuzzerStats) -> String {
    format!(
        "{}  score: {:.2}  pool: {}  exec/s: {}  cplx:{:.2}",
        stats.total_number_of_runs, stats.score, stats.pool_size, stats.exec_per_s, stats.avg_cplx
    )
}

fn event_to_string(event: FuzzerEvent) -> String {
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
    ChangeMessage(String),
}
pub enum OutMessage {}

impl ViewState for FuzzingView {
    type Update = self::Update;
    type InMessage = TuiMessage;
    type OutMessage = self::OutMessage;

    fn convert_in_message(&self, message: Self::InMessage) -> Option<Update> {
        match message {
            TuiMessage::AddInput { hash, input } => None,
            TuiMessage::RemoveInput { hash, input } => None,
            TuiMessage::ReportEvent { event, stats } => Some(Update::ChangeMessage(format!(
                "{}\n{}",
                event_to_string(event),
                stats_to_string(stats)
            ))),
        }
    }

    fn update(&mut self, u: Update) -> Option<OutMessage> {
        match u {
            Update::ChangeMessage(x) => {
                self.message = x;
                None
            }
        }
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

        let text = Text::from(self.message.as_str());
        let p = Paragraph::new(text)
            .block(Block::default().borders(Borders::ALL))
            .style(theme.default)
            .alignment(Alignment::Left)
            .wrap(Wrap { trim: true });

        frame.render_widget(p, bottom_chunks[0]);
    }
}
