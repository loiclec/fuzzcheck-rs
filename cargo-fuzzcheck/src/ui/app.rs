use std::path::PathBuf;

use tui::{Frame, backend::Backend};

use crate::project;

use crate::ui::initial;

use super::{events::Event, framework::{Either}};

pub struct State {
    pub phase: Phase,
    pub project: Option<project::Root>,
}

pub enum Phase {
    Initial(initial::State),
    Initialized,
    _Started,
    _Ended
}

pub enum Update {
    Initial(initial::Update),
    ChangePhase(Phase),
}

pub enum OutMessage {
    Quit
}

pub fn convert_initial_out_message(message: initial::OutMessage) -> Either<Update, OutMessage> {
    match message {
        initial::OutMessage::Initialized => {
            Either::Left(Update::ChangePhase(Phase::Initialized))
        }
        initial::OutMessage::Quit => {
            Either::Right(OutMessage::Quit)
        }
    }
}

impl State {
    pub fn new(root_path: PathBuf) -> Self {
        match project::Root::from_path(&root_path) {
            Ok(_project) => {
                panic!()
            }
            Err(_) => {
               State { 
                   phase: Phase::Initial(initial::State::new(root_path)),
                   project: None
               }
            }
        }
    }

    pub fn convert_in_message(&self, event: Event<()>) -> Option<Update> {
        match &self.phase {
            Phase::Initial(state) => {
                match event {
                    Event::UserInput(u) => {
                        state.convert_in_message(u).map(Update::Initial)
                    }
                    Event::_Subscription(_) => { 
                        None 
                    }
                }
            }
            Phase::Initialized => { None }
            Phase::_Started => { None }
            Phase::_Ended => { None }
        }
    }
    

    pub fn update(&mut self, u: Update) -> Option<OutMessage> {
        if let Update::ChangePhase(phase) = u {
            match phase {
                Phase::Initial(_) => { 
                    None 
                }
                Phase::Initialized => {
                    None
                }
                Phase::_Started => {
                    None
                }
                Phase::_Ended => {
                    None
                }
            }
        } else {
            match (&mut self.phase, u) {
                (Phase::Initial(state), Update::Initial(u)) => {
                    if let Some(out) = state.update(u) {
                        match convert_initial_out_message(out) {
                            Either::Left(u) => {
                                self.update(u)
                            }
                            Either::Right(out) => { 
                                Some(out) 
                            }
                        }
                    } else {
                        None
                    }
                }
                (Phase::Initialized, _) => { None }
                (Phase::_Started, _) => { None }
                (Phase::_Ended, _) => { None }
                _ => { None }
            }
        }
    }
    pub fn draw<B>(&mut self, frame: &mut Frame<B>) where B: Backend {
        match &mut self.phase {
            Phase::Initial(state) => { 
                state.draw(frame) 
            }
            Phase::Initialized => { }
            Phase::_Started => {}
            Phase::_Ended => {}
        }
    }
}
