use std::{path::PathBuf, rc::Rc};

use termion::event::Key;
use tui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    Frame,
};

use crate::project::{FullConfig, Root};

use super::{
    framework::{
        override_map, AnyView, Either, HorizontalMove, InnerFocusable, ParentView, SwitchFocus, Theme, ViewState,
    },
    horizontal_list_view::{self, HorizontalListView},
    run_fuzz::{self, RunFuzzView},
};

pub struct InitializedView {
    pub root: Rc<Root>,
    focus: Focus,
    fuzz_target_list: HorizontalListView,
    run_fuzz_views: Vec<RunFuzzView>,
}

impl InitializedView {
    pub fn new(root: Rc<Root>) -> Self {
        let fuzz_target_list = HorizontalListView::new("Fuzz Targets", fuzz_targets_from_root(&root).into_iter());

        let run_fuzz_views = fuzz_target_list
            .items
            .iter()
            .map(|fuzz_target| RunFuzzView::new(root.clone(), fuzz_target.clone()))
            .collect();
        let focus = Focus::Sidebar;

        let mut res = Self {
            root: root,
            focus,
            fuzz_target_list,
            run_fuzz_views,
        };
        res.update_focus(res.focus);
        res
    }
}

impl InitializedView {
    fn current_target_name(&self) -> Option<String> {
        if let Some(selected) = self.fuzz_target_list.state.selected() {
            Some(self.fuzz_target_list.items[selected].clone())
        } else {
            None
        }
    }
    fn current_run_fuzz_view(&self) -> Option<&RunFuzzView> {
        if let Some(selected) = self.fuzz_target_list.state.selected() {
            self.run_fuzz_views.get(selected)
        } else {
            None
        }
    }
    fn current_run_fuzz_view_as_mut(&mut self) -> Option<&mut RunFuzzView> {
        if let Some(selected) = self.fuzz_target_list.state.selected() {
            self.run_fuzz_views.get_mut(selected)
        } else {
            None
        }
    }
}

#[derive(Clone, Copy)]
pub enum Focus {
    Sidebar,
    Main,
}

pub enum Update {
    Sidebar(HorizontalMove),
    RunFuzz(run_fuzz::Update),
    SwitchFocus(Focus),
    SelectTarget(usize),
}

pub enum OutMessage {
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
}

impl InnerFocusable for InitializedView {
    type Focus = self::Focus;

    fn focus(&mut self) -> &mut Self::Focus {
        &mut self.focus
    }
    fn view_in_focus_ref(&self) -> Option<&dyn AnyView> {
        match self.focus {
            Focus::Sidebar => Some(&self.fuzz_target_list),
            Focus::Main => {
                if self.fuzz_target_list.state.selected().is_none() {
                    Some(&self.fuzz_target_list)
                } else {
                    Some(self.current_run_fuzz_view().unwrap())
                }
            }
        }
    }

    fn view_in_focus_mut(&mut self) -> Option<&mut dyn AnyView> {
        match self.focus {
            Focus::Sidebar => Some(&mut self.fuzz_target_list),
            Focus::Main => {
                if self.fuzz_target_list.state.selected().is_none() {
                    Some(&mut self.fuzz_target_list)
                } else {
                    Some(self.current_run_fuzz_view_as_mut().unwrap())
                }
            }
        }
    }

    fn focus_after_switch(&self, sf: SwitchFocus) -> Option<Self::Focus> {
        match sf {
            SwitchFocus::Up => match self.focus {
                Focus::Sidebar => None,
                Focus::Main => Some(Focus::Sidebar),
            },
            SwitchFocus::Right => None,
            SwitchFocus::Down => match self.focus {
                Focus::Sidebar => Some(Focus::Main),
                Focus::Main => None,
            },
            SwitchFocus::Left => None,
            SwitchFocus::Next => match self.focus {
                Focus::Sidebar => Some(Focus::Main),
                Focus::Main => None,
            },
            SwitchFocus::Previous => match self.focus {
                Focus::Sidebar => None,
                Focus::Main => Some(Focus::Sidebar),
            },
            SwitchFocus::In => match self.focus {
                Focus::Sidebar => Some(Focus::Main),
                Focus::Main => None,
            },
            SwitchFocus::Out => match self.focus {
                Focus::Sidebar => None,
                Focus::Main => Some(Focus::Sidebar),
            },
        }
    }
}

impl AnyView for InitializedView {
    fn focus(&mut self) {}
    fn unfocus(&mut self) {}

    fn key_bindings(&self) -> Vec<(Key, String)> {
        let mut map = Vec::new();
        match self.focus {
            Focus::Sidebar => {
                map.push((Key::Char('\n'), "configure fuzz target".to_string()));
            }
            Focus::Main => {}
        }
        if let Some(v) = self.view_in_focus_ref() {
            override_map(&mut map, v.key_bindings())
        }
        map
    }
}

impl ViewState for InitializedView {
    type Update = self::Update;
    type InMessage = Key;
    type OutMessage = self::OutMessage;

    fn convert_in_message(&self, message: Self::InMessage) -> Option<Self::Update> {
        let new_focus = SwitchFocus::from_standard_key(&message).and_then(|sf| self.focus_after_switch(sf));
        match self.focus {
            Focus::Sidebar => Self::handle_child_in_message(&self.fuzz_target_list, &message)
                .or_else(|| new_focus.map(Update::SwitchFocus)),
            Focus::Main => {
                if let Some(run_fuzz) = self.current_run_fuzz_view() {
                    Self::handle_child_in_message(run_fuzz, &message).or_else(|| new_focus.map(Update::SwitchFocus))
                } else {
                    Some(Update::SwitchFocus(Focus::Sidebar))
                }
            }
        }
    }

    fn update(&mut self, u: Self::Update) -> Option<Self::OutMessage> {
        match u {
            Update::Sidebar(u) => self
                .fuzz_target_list
                .update(u)
                .and_then(|out| <Self as ParentView<HorizontalListView>>::handle_child_out_message(self, out)),
            Update::RunFuzz(u) => self
                .current_run_fuzz_view_as_mut()
                .and_then(|run_fuzz| run_fuzz.update(u))
                .and_then(|out| <Self as ParentView<RunFuzzView>>::handle_child_out_message(self, out)),
            Update::SwitchFocus(f) => {
                self.update_focus(f);
                None
            }
            Update::SelectTarget(_target) => {
                //self.run_fuzz = Some(RunFuzzView::new(self.root.clone(), target));
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
            .constraints([Constraint::Length(3), Constraint::Min(0)].as_ref())
            .split(area);

        self.fuzz_target_list.draw(frame, theme, chunks[0]);

        if let Some(run_fuzz) = self.current_run_fuzz_view() {
            run_fuzz.draw(frame, theme, chunks[1])
        }
    }
}

impl ParentView<HorizontalListView> for InitializedView {
    fn convert_child_update(update: <HorizontalListView as ViewState>::Update) -> Self::Update {
        Self::Update::Sidebar(update)
    }

    fn convert_to_child_in_message(message: &Self::InMessage) -> Option<HorizontalMove> {
        HorizontalMove::from(message)
    }

    fn convert_child_out_message(
        &self,
        message: horizontal_list_view::OutMessage,
    ) -> super::framework::Either<Update, OutMessage> {
        match message {
            horizontal_list_view::OutMessage::Select(target) => Either::Left(Update::SelectTarget(target)),
        }
    }
}

impl ParentView<RunFuzzView> for InitializedView {
    fn convert_child_update(update: run_fuzz::Update) -> Self::Update {
        Self::Update::RunFuzz(update)
    }

    fn convert_to_child_in_message(message: &Self::InMessage) -> Option<<RunFuzzView as ViewState>::InMessage> {
        Some(message.clone())
    }

    fn convert_child_out_message(&self, message: run_fuzz::OutMessage) -> super::framework::Either<Update, OutMessage> {
        match message {
            run_fuzz::OutMessage::StartFuzzing(config) => Either::Right(OutMessage::StartFuzzing {
                root: self.root.clone(),
                target_name: self.current_target_name().unwrap(),
                config,
            }),
            run_fuzz::OutMessage::BuildFuzzTarget(config) => Either::Right(OutMessage::BuildFuzzTarget {
                root: self.root.clone(),
                target_name: self.current_target_name().unwrap(),
                config,
            }),
        }
    }
}

fn fuzz_targets_from_root(root: &Root) -> Vec<String> {
    let mut targets = root
        .fuzz
        .non_instrumented
        .fuzz_targets
        .targets
        .keys()
        .map(|k| {
            let target = PathBuf::from(k);
            target.file_stem().unwrap().to_str().unwrap().to_string()
        })
        .collect::<Vec<_>>();
    targets.sort();
    targets
}
