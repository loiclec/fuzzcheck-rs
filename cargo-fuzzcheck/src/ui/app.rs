use std::{path::PathBuf, rc::Rc, sync::mpsc::Sender};

use termion::event::Key;
use tui::{backend::Backend, layout::Rect, Frame};

use crate::project::{self, Root};

use crate::ui::framework::ViewState;
use crate::ui::preinit;

use super::{
    error_view,
    events::Event,
    framework::{Either, ParentView, Theme},
    fuzz_target_comm::FuzzingEvent,
    fuzzing, initialized,
};

pub struct State {
    pub root_path: PathBuf,
    pub phase: Phase,
    pub sender: Sender<Event<FuzzingEvent>>,
}

impl State {
    pub fn new(root_path: PathBuf, sender: Sender<Event<FuzzingEvent>>) -> Self {
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
    _Ended,
}

pub enum Update {
    Error(error_view::Update),
    PreInit(preinit::Update),
    Initialized(initialized::Update),
    ChangePhase(Phase),
}

pub enum OutMessage {
    Quit,
}

impl ViewState for State {
    type Update = self::Update;

    type InMessage = Event<FuzzingEvent>;

    type OutMessage = self::OutMessage;

    fn convert_in_message(&self, message: Self::InMessage) -> Option<Self::Update> {
        match &self.phase {
            Phase::PreInit(state) => Self::handle_child_in_message(state, message),
            Phase::Error(state) => Self::handle_child_in_message(state, message),
            Phase::Initialized(state) => Self::handle_child_in_message(state, message),
            Phase::Fuzzing(state) => todo!(),
            Phase::_Ended => None,
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
                (Phase::Fuzzing(state), _) => {
                    todo!()
                }
                (Phase::_Ended, _) => None,
                _ => None,
            }
        }
    }
    fn draw<B>(&self, frame: &mut Frame<B>, theme: &Theme, area: Rect)
    where
        B: Backend,
    {
        match &self.phase {
            Phase::PreInit(state) => state.draw(frame, theme, area),
            Phase::Error(state) => state.draw(frame, theme, area),
            Phase::Initialized(state) => state.draw(frame, theme, area),
            Phase::Fuzzing(state) => {
                todo!()
            }
            Phase::_Ended => {}
        }
    }
}

impl ParentView<preinit::PreInitView> for State {
    fn convert_child_update(update: <preinit::PreInitView as ViewState>::Update) -> Self::Update {
        Self::Update::PreInit(update)
    }

    fn convert_to_child_in_message(message: Self::InMessage) -> Option<<preinit::PreInitView as ViewState>::InMessage> {
        match message {
            Event::UserInput(u) => Some(u),
            Event::Subscription(_) => None,
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
    fn convert_to_child_in_message(message: Self::InMessage) -> Option<Key> {
        match message {
            Event::UserInput(u) => Some(u),
            Event::Subscription(_) => None,
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
    fn convert_to_child_in_message(message: Self::InMessage) -> Option<Key> {
        match message {
            Event::UserInput(u) => Some(u),
            Event::Subscription(_) => None,
        }
    }
    fn convert_child_out_message(&self, message: initialized::OutMessage) -> Either<Update, OutMessage> {
        match message {
            initialized::OutMessage::Run(args) => {
                todo!()
            }
        }
    }
}
