use std::path::PathBuf;

use tui::{Frame, backend::Backend};

use crate::project::{self, Root};

use crate::ui::preinit;

use super::{error_view, initialized, events::Event, framework::{Either}};

pub struct State {
    pub root_path: PathBuf,
    pub phase: Phase,
}

pub enum Phase {
    Error(error_view::State),
    PreInit(preinit::State),
    Initialized(initialized::State),
    _Started,
    _Ended
}

pub enum Update {
    Error(error_view::Update),
    PreInit(preinit::Update),
    Initialized(initialized::Update),
    ChangePhase(Phase),
}

pub enum OutMessage {
    Quit
}

impl State {

    pub fn convert_preinit_out_message(&self, message: preinit::OutMessage) -> Either<Update, OutMessage> {
        match message {
            preinit::OutMessage::Initialized => {
                match Root::from_path(&self.root_path) {
                    Ok(root) => {
                        let state = initialized::State::new(root);
                        Either::Left(Update::ChangePhase(Phase::Initialized(state)))
                    }
                    Err(err) => {
                        let state =  error_view::State::new(Box::new(err));
                        Either::Left(Update::ChangePhase(Phase::Error(state)))
                    }
                }
            }
            preinit::OutMessage::Quit => {
                Either::Right(OutMessage::Quit)
            }
            preinit::OutMessage::Error(err) => {
                Either::Left(Update::ChangePhase(Phase::Error(error_view::State::new(Box::new(err)))))
            }
        }
    }
    
    pub fn convert_error_out_message(&self, _message: error_view::OutMessage) -> Either<Update, OutMessage> {
        Either::Right(OutMessage::Quit)
    }
    

    pub fn new(root_path: PathBuf) -> Self {
        match project::Root::from_path(&root_path) {
            Ok(root) => {
                let state = initialized::State::new(root);
                State {
                    root_path: root_path.clone(),
                    phase: Phase::Initialized(state),
                }
            }
            Err(_) => {
                match preinit::State::new(&root_path) {
                    Ok(state) => {
                        State {
                            root_path: root_path.clone(),
                            phase: Phase::PreInit(state),
                        }
                    }
                    Err(err) => {
                        let state = error_view::State::new(Box::new(err));
                        State {
                            root_path: root_path.clone(),
                            phase: Phase::Error(state),
                        }
                    }
                }
            }
        }
    }

    pub fn convert_in_message(&self, event: Event<()>) -> Option<Update> {
        match &self.phase {
            Phase::PreInit(state) => {
                match event {
                    Event::UserInput(u) => {
                        state.convert_in_message(u).map(Update::PreInit)
                    }
                    Event::_Subscription(_) => { 
                        None 
                    }
                }
            }
            Phase::Error(state) => { 
                match event {
                    Event::UserInput(u) => {
                        state.convert_in_message(u).map(Update::Error)
                    }
                    Event::_Subscription(_) => { 
                        None 
                    }
                }
             }
            Phase::Initialized(state) => { 
                match event {
                    Event::UserInput(u) => {
                        state.convert_in_message(u).map(Update::Initialized)
                    }
                    Event::_Subscription(_) => { 
                        None 
                    }
                }
             }
            Phase::_Started => { None }
            Phase::_Ended => { None }
        }
    }

    pub fn update(&mut self, u: Update) -> Option<OutMessage> {
        if let Update::ChangePhase(phase) = u {
            self.phase = phase;
            None
        } else {
            match (&mut self.phase, u) {
                (Phase::Error(state), Update::Error(u)) => { 
                    if let Some(out) = state.update(u) {
                        match self.convert_error_out_message(out) {
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
                (Phase::PreInit(state), Update::PreInit(u)) => {
                    if let Some(out) = state.update(u) {
                        match self.convert_preinit_out_message(out) {
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
                (Phase::Initialized(state), _) => { None }
                (Phase::_Started, _) => { None }
                (Phase::_Ended, _) => { None }
                _ => { None }
            }
        }
    }
    pub fn draw<B>(&mut self, frame: &mut Frame<B>) where B: Backend {
        match &mut self.phase {
            Phase::PreInit(state) => { 
                state.draw(frame) 
            }
            Phase::Error(state) => {
                state.draw(frame)
            }
            Phase::Initialized(state) => {
                state.draw(frame)
             }
            Phase::_Started => {}
            Phase::_Ended => {}
        }
    }
}
