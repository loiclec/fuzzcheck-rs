use std::{path::PathBuf, rc::Rc, sync::mpsc::Sender};

use project::FullConfig;
use termion::event::Key;
use tui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    Frame,
};

use crate::project::{self, Root};

use crate::ui::framework::ViewState;
use crate::ui::preinit;

use fuzzcheck_common::ipc::TuiMessage;

use super::{
    error_view,
    events::{Event, EXIT_KEY},
    framework::{override_map, AnyView, Either, ExplainKeyBindingView, ParentView, Theme},
    fuzzing, initialized,
};

pub struct State {
    pub root_path: PathBuf,
    pub phase: Phase,
    pub sender: Sender<Event<TuiMessage>>,
}

impl State {
    pub fn new(root_path: PathBuf, sender: Sender<Event<TuiMessage>>) -> Self {
        match project::Root::from_path(&root_path) {
            Ok(root) => {
                let state = initialized::InitializedView::new(Rc::new(root));
                State {
                    root_path: root_path.clone(),
                    phase: Phase::Initialized(state),
                    sender,
                }
            }
            Err(_) => match preinit::PreInitView::new(&root_path) {
                Ok(state) => State {
                    root_path: root_path.clone(),
                    phase: Phase::PreInit(state),
                    sender,
                },
                Err(err) => {
                    let state = error_view::ErrorView::new(Box::new(err));
                    State {
                        root_path: root_path.clone(),
                        phase: Phase::Error(state),
                        sender,
                    }
                }
            },
        }
    }
}

pub enum Phase {
    Error(error_view::ErrorView),
    PreInit(preinit::PreInitView),
    Initialized(initialized::InitializedView),
    Fuzzing(fuzzing::FuzzingView),
}

pub enum Update {
    Error(error_view::Update),
    PreInit(preinit::Update),
    Initialized(initialized::Update),
    Fuzzing(fuzzing::Update),
    ChangePhase(Phase),
}

pub enum OutMessage {
    Quit,
    BuildFuzzTarget {
        root: Rc<Root>,
        target_name: String,
        config: FullConfig,
    },
    StartFuzzing {
        root: Rc<Root>,
        target_name: String,
        config: FullConfig,
    },
    PauseFuzzer,
    UnPauseFuzzer,
    UnPauseFuzzerUntilNextEvent,
    StopFuzzer,
}

impl AnyView for State {
    fn focus(&mut self) {}

    fn unfocus(&mut self) {}

    fn key_bindings(&self) -> Vec<(Key, String)> {
        let mut map = Vec::new();
        map.push((EXIT_KEY, "quit".to_string()));
        let merging = match &self.phase {
            Phase::Error(v) => v.key_bindings(),
            Phase::PreInit(v) => v.key_bindings(),
            Phase::Initialized(v) => v.key_bindings(),
            Phase::Fuzzing(v) => v.key_bindings(),
        };
        override_map(&mut map, merging);
        map
    }
}

impl ViewState for State {
    type Update = self::Update;

    type InMessage = Event<TuiMessage>;

    type OutMessage = self::OutMessage;

    fn convert_in_message(&self, message: Self::InMessage) -> Option<Self::Update> {
        match &self.phase {
            Phase::PreInit(state) => Self::handle_child_in_message(state, &message),
            Phase::Error(state) => Self::handle_child_in_message(state, &message),
            Phase::Initialized(state) => Self::handle_child_in_message(state, &message),
            Phase::Fuzzing(state) => Self::handle_child_in_message(state, &message),
        }
    }

    fn update(&mut self, u: Self::Update) -> Option<Self::OutMessage> {
        if let Update::ChangePhase(phase) = u {
            self.phase = phase;
            None
        } else {
            match (&mut self.phase, u) {
                (Phase::Error(state), Update::Error(u)) => state
                    .update(u)
                    .and_then(|out| <Self as ParentView<error_view::ErrorView>>::handle_child_out_message(self, out)),
                (Phase::PreInit(state), Update::PreInit(u)) => state
                    .update(u)
                    .and_then(|out| <Self as ParentView<preinit::PreInitView>>::handle_child_out_message(self, out)),
                (Phase::Initialized(state), Update::Initialized(u)) => state.update(u).and_then(|out| {
                    <Self as ParentView<initialized::InitializedView>>::handle_child_out_message(self, out)
                }),
                (Phase::Fuzzing(state), Update::Fuzzing(u)) => state
                    .update(u)
                    .and_then(|out| <Self as ParentView<fuzzing::FuzzingView>>::handle_child_out_message(self, out)),
                _ => None,
            }
        }
    }
    fn draw<B>(&self, frame: &mut Frame<B>, theme: &Theme, area: Rect)
    where
        B: Backend,
    {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(2), Constraint::Min(0)].as_ref())
            .split(area);
        let key_bindings = match &self.phase {
            Phase::PreInit(state) => state.key_bindings(),
            Phase::Error(state) => state.key_bindings(),
            Phase::Initialized(state) => state.key_bindings(),
            Phase::Fuzzing(state) => state.key_bindings(),
        };
        let kbview = ExplainKeyBindingView::new(key_bindings);
        kbview.draw(frame, theme, chunks[0]);

        match &self.phase {
            Phase::PreInit(state) => state.draw(frame, theme, chunks[1]),
            Phase::Error(state) => state.draw(frame, theme, chunks[1]),
            Phase::Initialized(state) => state.draw(frame, theme, chunks[1]),
            Phase::Fuzzing(state) => state.draw(frame, theme, chunks[1]),
        }
    }
}

impl ParentView<preinit::PreInitView> for State {
    fn convert_child_update(update: <preinit::PreInitView as ViewState>::Update) -> Self::Update {
        Self::Update::PreInit(update)
    }

    fn convert_to_child_in_message(
        message: &Self::InMessage,
    ) -> Option<<preinit::PreInitView as ViewState>::InMessage> {
        match message {
            Event::UserInput(u) => Some(u.clone()),
            Event::Subscription(_) => None,
            Event::Tick => None,
        }
    }

    fn convert_child_out_message(
        &self,
        message: <preinit::PreInitView as ViewState>::OutMessage,
    ) -> Either<Self::Update, Self::OutMessage> {
        match message {
            preinit::OutMessage::Initialized => match Root::from_path(&self.root_path) {
                Ok(root) => {
                    let state = initialized::InitializedView::new(Rc::new(root));
                    Either::Left(Update::ChangePhase(Phase::Initialized(state)))
                }
                Err(err) => {
                    let state = error_view::ErrorView::new(Box::new(err));
                    Either::Left(Update::ChangePhase(Phase::Error(state)))
                }
            },
            preinit::OutMessage::Quit => Either::Right(OutMessage::Quit),
            preinit::OutMessage::Error(err) => Either::Left(Update::ChangePhase(Phase::Error(
                error_view::ErrorView::new(Box::new(err)),
            ))),
        }
    }
}
impl ParentView<error_view::ErrorView> for State {
    fn convert_child_update(update: <error_view::ErrorView as ViewState>::Update) -> Self::Update {
        Self::Update::Error(update)
    }
    fn convert_to_child_in_message(message: &Self::InMessage) -> Option<Key> {
        match message {
            Event::UserInput(u) => Some(u.clone()),
            Event::Subscription(_) => None,
            Event::Tick => None,
        }
    }

    fn convert_child_out_message(&self, _message: error_view::OutMessage) -> Either<Update, OutMessage> {
        Either::Right(OutMessage::Quit)
    }
}
impl ParentView<initialized::InitializedView> for State {
    fn convert_child_update(update: <initialized::InitializedView as ViewState>::Update) -> Self::Update {
        Self::Update::Initialized(update)
    }
    fn convert_to_child_in_message(message: &Self::InMessage) -> Option<Key> {
        match message {
            Event::UserInput(u) => Some(u.clone()),
            Event::Subscription(_) => None,
            Event::Tick => None,
        }
    }
    fn convert_child_out_message(&self, message: initialized::OutMessage) -> Either<Update, OutMessage> {
        match message {
            initialized::OutMessage::StartFuzzing {
                root,
                target_name,
                config,
            } => Either::Right(OutMessage::StartFuzzing {
                root,
                target_name,
                config,
            }),
            initialized::OutMessage::BuildFuzzTarget {
                root,
                target_name,
                config,
            } => Either::Right(OutMessage::BuildFuzzTarget {
                root,
                target_name,
                config,
            }),
        }
    }
}
impl ParentView<fuzzing::FuzzingView> for State {
    fn convert_child_update(update: <fuzzing::FuzzingView as ViewState>::Update) -> Self::Update {
        Self::Update::Fuzzing(update)
    }
    fn convert_to_child_in_message(
        message: &Self::InMessage,
    ) -> Option<<fuzzing::FuzzingView as ViewState>::InMessage> {
        match message {
            Event::UserInput(x) => Some(fuzzing::InMessage::Key(x.clone())),
            Event::Subscription(m) => Some(fuzzing::InMessage::TuiMessage(m.clone())),
            Event::Tick => None,
        }
    }
    fn convert_child_out_message(
        &self,
        message: <fuzzing::FuzzingView as ViewState>::OutMessage,
    ) -> Either<Update, OutMessage> {
        match message {
            fuzzing::OutMessage::PauseFuzzer => Either::Right(OutMessage::PauseFuzzer),
            fuzzing::OutMessage::UnPauseFuzzer => Either::Right(OutMessage::UnPauseFuzzer),
            fuzzing::OutMessage::StopFuzzer => Either::Right(OutMessage::StopFuzzer),
            fuzzing::OutMessage::UnPauseFuzzerUntilNextEvent => Either::Right(OutMessage::UnPauseFuzzerUntilNextEvent),
        }
    }
}
