use std::{error::Error, fmt::Display, rc::Rc, write};

use fuzzcheck_common::arg::{CommandLineArguments};

/**
    This view presents the default configuration for a particular fuzz target,
    gives the opportunity to change some of the settings, and then launch the
    fuzzcheck on the target.
*/
use termion::event::Key;
use tui::{
    backend::Backend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    text::{Span, Spans},
    widgets::{Block, BorderType, Borders, Paragraph, Wrap},
    Frame,
};

use crate::project::{FullConfig, Root};

use super::{
    framework::{Either, Focusable, InnerFocusable, ParentView, Theme, ViewState},
    text_field_view::TextFieldView,
};

#[derive(Debug)]
struct FloatEqualZeroError;
impl Display for FloatEqualZeroError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "float should be greater than 0.0")
    }
}
impl Error for FloatEqualZeroError {}

pub enum ArgParseError {
    MaxInputComplexity(Box<dyn Error>),
}
impl Display for ArgParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ArgParseError::MaxInputComplexity(err) => {
                write!(
                    f,
                    "The value for 'max input complexity' could not be parsed.\nParsing error: {}",
                    err
                )
            }
        }
    }
}

pub struct RunFuzzView {
    pub root: Rc<Root>,
    fuzz_target: String,
    initial_config: FullConfig,
    final_config: FullConfig,
    error: Option<ArgParseError>,
    max_cplx_field: TextFieldView,
    focus: Focus,
    focused: bool,
}

impl RunFuzzView {
    pub fn new(root: Rc<Root>, fuzz_target: String) -> Self {
        let config = root.full_config(&fuzz_target, &CommandLineArguments::default());
        let max_cplx = format!("{}", config.max_cplx);
        if matches!(config.command, crate::project::FullFuzzerCommand::MinifyCorpus{..}) {
            panic!()
        }
        Self {
            root,
            fuzz_target,
            initial_config: config.clone(),
            final_config: config,
            error: None,
            max_cplx_field: TextFieldView::new(max_cplx),
            focus: Focus::RunButton,
            focused: false,
        }
    }
}

#[derive(Clone, Copy)]
pub enum Focus {
    MaxInputComplexity,
    RunButton,
}
impl Focus {
    fn next(self) -> Option<Focus> {
        match self {
            Focus::MaxInputComplexity => Some(Focus::RunButton),
            Focus::RunButton => None,
        }
    }
    fn prev(self) -> Option<Focus> {
        match self {
            Focus::MaxInputComplexity => None,
            Focus::RunButton => Some(Focus::MaxInputComplexity),
        }
    }
}

impl Focusable for RunFuzzView {
    fn focus(&mut self) {
        self.focused = true;
    }

    fn unfocus(&mut self) {
        self.focused = false;
    }
}
impl InnerFocusable for RunFuzzView {
    type Focus = self::Focus;

    fn focus(&mut self) -> &mut Self::Focus {
        &mut self.focus
    }

    fn view_in_focus(&mut self) -> Option<&mut dyn Focusable> {
        match self.focus {
            Focus::MaxInputComplexity => Some(&mut self.max_cplx_field),
            Focus::RunButton => None,
        }
    }
}

pub enum Update {
    StartFuzzing,
    SwitchFocus(Focus),
    MaxInputComplexityView(<TextFieldView as ViewState>::Update),
    SetMaxInputComplexity(String),
}

pub enum OutMessage {
    StartFuzzing(FullConfig),
}

impl ViewState for RunFuzzView {
    type Update = self::Update;
    type InMessage = Key;
    type OutMessage = self::OutMessage;

    fn convert_in_message(&self, message: Self::InMessage) -> Option<Self::Update> {
        match self.focus {
            Focus::RunButton => match message {
                Key::Char('\n') => {
                    if self.error.is_none() {
                        return Some(Update::StartFuzzing);
                    } else {
                        return None;
                    }
                }
                _ => {}
            },
            Focus::MaxInputComplexity => {
                if let Some(u) = Self::handle_child_in_message(&self.max_cplx_field, message) {
                    return Some(u);
                }
            }
        }
        match message {
            Key::Up | Key::Left => self.focus.prev().map(Update::SwitchFocus),
            Key::Down | Key::Right => self.focus.next().map(Update::SwitchFocus),
            Key::BackTab => self.focus.prev().map(Update::SwitchFocus),
            Key::Char('\t') => self.focus.next().map(Update::SwitchFocus),
            Key::Esc => {
                None //Some(Update::Unfocus)
            }
            _ => None,
        }
    }

    fn update(&mut self, u: Self::Update) -> Option<Self::OutMessage> {
        match u {
            Update::StartFuzzing => Some(OutMessage::StartFuzzing(self.final_config.clone())),
            Update::SwitchFocus(f) => {
                self.update_focus(f);
                None
            }
            Update::MaxInputComplexityView(u) => self
                .max_cplx_field
                .update(u)
                .and_then(|out| self.handle_child_out_message(out)),

            Update::SetMaxInputComplexity(s) => {
                if s.is_empty() {
                    self.final_config.max_cplx = self.initial_config.max_cplx;
                    self.error = None;
                } else {
                    match str::parse::<f64>(s.trim()) {
                        Ok(cplx) => {
                            if cplx <= 0.0 {
                                self.final_config.max_cplx = self.initial_config.max_cplx;
                                self.error = Some(ArgParseError::MaxInputComplexity(Box::new(FloatEqualZeroError)))
                            } else {
                                self.final_config.max_cplx = cplx;
                                self.error = None;
                            }
                        }
                        Err(e) => {
                            self.final_config.max_cplx = self.initial_config.max_cplx;
                            self.error = Some(ArgParseError::MaxInputComplexity(Box::new(e)))
                        }
                    }
                }
                None
            }
        }
    }

    fn draw<B>(&self, frame: &mut Frame<B>, theme: &Theme, area: Rect)
    where
        B: Backend,
    {
        let inner_theme = if self.focused {
            Theme::primary()
        } else {
            Theme::secondary()
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .title(format!("Launch Fuzzcheck on {}", self.fuzz_target))
            .style(if self.focused {
                theme.block_highlight
            } else {
                theme.default
            });

        let inner_area = block.inner(area);

        frame.render_widget(block, area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                [
                    Constraint::Length(3),
                    Constraint::Length(3),
                    Constraint::Length(3),
                    Constraint::Min(0),
                ]
                .as_ref(),
            )
            .split(inner_area);

        let max_cplx_block = Block::default()
            .title("Maximum input complexity")
            .borders(Borders::ALL)
            .style(inner_theme.default);

        let inner_max_cplx_block = max_cplx_block.inner(chunks[0]);
        frame.render_widget(max_cplx_block, chunks[0]);

        self.max_cplx_field.draw(frame, &inner_theme, inner_max_cplx_block);

        match &self.error {
            Some(err) => {
                let error_text = Paragraph::new(format!("{}", err))
                    .style(theme.error)
                    .wrap(Wrap { trim: true });
                frame.render_widget(error_text, chunks[1]);

                let run_text = Paragraph::new("Start Fuzzing (disabled)").block(
                    Block::default()
                        .borders(Borders::ALL)
                        .style(inner_theme.disabled)
                        .border_type(if matches!(self.focus, Focus::RunButton) {
                            BorderType::Double
                        } else {
                            BorderType::Plain
                        }),
                );

                frame.render_widget(run_text, chunks[2]);
            }
            None => {
                let command_text = Paragraph::new(vec![
                    Spans::from(Span::styled(
                        "If you start fuzzing now, the following command will be run:",
                        inner_theme.default,
                    )),
                    Spans::from(Span::styled(
                        crate::strings_from_config(&self.final_config).join(" "),
                        inner_theme.emphasis,
                    )),
                ])
                .style(theme.default)
                .wrap(Wrap { trim: true })
                .alignment(Alignment::Center);
                frame.render_widget(command_text, chunks[1]);

                let run_text = Paragraph::new("Start Fuzzing").block(Block::default().borders(Borders::ALL).style(
                    if matches!(self.focus, Focus::RunButton) {
                        inner_theme.highlight
                    } else {
                        inner_theme.default
                    },
                ));

                frame.render_widget(run_text, chunks[2]);
            }
        }
    }
}

impl ParentView<TextFieldView> for RunFuzzView {
    fn convert_child_update(update: <TextFieldView as ViewState>::Update) -> Self::Update {
        self::Update::MaxInputComplexityView(update)
    }

    fn convert_to_child_in_message(message: Self::InMessage) -> Option<<TextFieldView as ViewState>::InMessage> {
        Some(message)
    }

    fn convert_child_out_message(
        &self,
        message: <TextFieldView as ViewState>::OutMessage,
    ) -> super::framework::Either<Self::Update, Self::OutMessage> {
        match message {
            super::text_field_view::OutMessage::Edited(s) => Either::Left(self::Update::SetMaxInputComplexity(s)),
        }
    }
}

// impl ParentView<SelectableList> for RunFuzzView {
//     fn convert_out_message(&self, message: selectable_list::OutMessage) -> super::framework::Either<Update, OutMessage> {
//         match message {
//             selectable_list::OutMessage::Select(target) => {
//                 Either::Left(Update::SelectTarget(target))
//             }
//         }
//     }
// }

// fn fuzz_targets_from_root(root: &Root) -> &HashMap<OsString, Vec<u8>> {
//     &root.fuzz.non_instrumented.fuzz_targets.targets
// }
