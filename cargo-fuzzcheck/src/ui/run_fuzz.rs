use std::rc::Rc;

use fuzzcheck_common::arg::{options_parser, CommandLineArguments, DEFAULT_ARGUMENTS};
use getopts::Options;
/**
    This view presents the default configuration for a particular fuzz target,
    gives the opportunity to change some of the settings, and then launch the
    fuzzcheck on the target.
*/
use termion::event::Key;
use tui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    widgets::{Block, BorderType, Borders, Paragraph, Row, Table},
    Frame,
};

use crate::project::Root;

use super::{
    framework::{
        Theme, Focusable, InnerFocusable, ParentView, ViewState,
    },
    text_field_view::TextFieldView,
};

pub struct RunFuzzView {
    pub root: Rc<Root>,
    fuzz_target: String,
    max_input_cplx_field: TextFieldView,
    focus: Focus,
    focused: bool,
}

impl RunFuzzView {
    pub fn new(root: Rc<Root>, fuzz_target: String) -> Self {
        let options_parser = options_parser();
        let args = CommandLineArguments::from_parser(&options_parser, &["fuzz".to_string()]).unwrap();
        let config = root.fuzz.config_toml.resolved_config(&fuzz_target);
        let args = config.resolve_arguments(&args);
        let args = args.resolved(DEFAULT_ARGUMENTS);

        let max_input_cplx = format!("{}", args.max_input_cplx);

        Self {
            root,
            fuzz_target,
            max_input_cplx_field: TextFieldView::new(max_input_cplx),
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
            Focus::MaxInputComplexity => Some(&mut self.max_input_cplx_field),
            Focus::RunButton => None,
        }
    }
}

pub enum Update {
    Run,
}

pub enum OutMessage {}

impl ViewState for RunFuzzView {
    type Update = self::Update;
    type InMessage = Key;
    type OutMessage = self::OutMessage;

    fn convert_in_message(&self, message: Self::InMessage) -> Option<Self::Update> {
        match self.focus {
            Focus::RunButton => match message {
                Key::Char('\n') => Some(Update::Run),
                _ => None,
            },

            Focus::MaxInputComplexity => None,
        }
    }

    fn update(&mut self, u: Self::Update) -> Option<Self::OutMessage> {
        match u {
            Update::Run => None,
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
            .constraints([Constraint::Length(3), Constraint::Length(3), Constraint::Min(0)].as_ref())
            .split(inner_area);

        let max_cplx_block = Block::default()
            .title("Maximum input complexity")
            .borders(Borders::ALL)
            .style(inner_theme.default);

        let inner_max_cplx_block = max_cplx_block.inner(chunks[0]);
        frame.render_widget(max_cplx_block, chunks[0]);

        self.max_input_cplx_field.draw(frame, &inner_theme, inner_max_cplx_block);

        let run_text = Paragraph::new("Start Fuzzing").block(Block::default().borders(Borders::ALL).style(
            if matches!(self.focus, Focus::RunButton) {
                inner_theme.highlight
            } else {
                inner_theme.default
            },
        ));

        frame.render_widget(run_text, chunks[1]);

        // let table = Table::new(vec![
        //     Row::new(vec!["max input complexity", max_input_cplx.as_ref()]),
        // ]).style(default_style())
        //     .column_spacing(2)
        //     .widths(&[Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)])
        //     .block(Block::default().title("Configuration").borders(Borders::ALL));

        // frame.render_widget(table, inner_area);
    }
}

impl ParentView<TextFieldView> for RunFuzzView {
    fn convert_child_update(update: <TextFieldView as ViewState>::Update) -> Self::Update {
        todo!()
    }

    fn convert_to_child_in_message(message: Self::InMessage) -> Option<<TextFieldView as ViewState>::InMessage> {
        todo!()
    }

    fn convert_child_out_message(
        &self,
        message: <TextFieldView as ViewState>::OutMessage,
    ) -> super::framework::Either<Self::Update, Self::OutMessage> {
        todo!()
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
