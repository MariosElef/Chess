use std::rc::Rc;

use gpui_kit::{
    assets::IconName,
    component::{
        Disableable as _,
        button::{Button, ButtonVariants},
        h_flex,
    },
    *,
};

/// One move through a game's history.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NavStep {
    Start,
    Back,
    Forward,
    Live,
}

type OnStep = Rc<dyn Fn(NavStep, &mut Window, &mut App)>;

/// ⏮ ◀ ▶ ⏭ — the buttons at either end disable themselves when there is nowhere to go.
#[derive(IntoElement)]
pub struct NavigationBar {
    can_go_back: bool,
    can_go_forward: bool,
    on_step: OnStep,
}

impl NavigationBar {
    pub fn new(
        can_go_back: bool,
        can_go_forward: bool,
        on_step: impl Fn(NavStep, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            can_go_back,
            can_go_forward,
            on_step: Rc::new(on_step),
        }
    }
}

impl RenderOnce for NavigationBar {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        h_flex()
            .gap_2()
            .child(self.button(NavStep::Start, !self.can_go_back))
            .child(self.button(NavStep::Back, !self.can_go_back))
            .child(self.button(NavStep::Forward, !self.can_go_forward))
            .child(self.button(NavStep::Live, !self.can_go_forward))
    }
}

impl NavigationBar {
    fn button(&self, step: NavStep, disabled: bool) -> Button {
        let on_step = Rc::clone(&self.on_step);
        Button::new(step_id(step))
            .ghost()
            .icon(step_icon(step))
            .tooltip(step_tooltip(step))
            .disabled(disabled)
            .on_click(move |_, window, cx| on_step(step, window, cx))
    }
}

fn step_id(step: NavStep) -> &'static str {
    match step {
        NavStep::Start => "nav-start",
        NavStep::Back => "nav-back",
        NavStep::Forward => "nav-forward",
        NavStep::Live => "nav-live",
    }
}

fn step_icon(step: NavStep) -> IconName {
    match step {
        NavStep::Start => IconName::ChevronsLeft,
        NavStep::Back => IconName::ChevronLeft,
        NavStep::Forward => IconName::ChevronRight,
        NavStep::Live => IconName::ChevronsRight,
    }
}

fn step_tooltip(step: NavStep) -> &'static str {
    match step {
        NavStep::Start => "First position",
        NavStep::Back => "Previous position",
        NavStep::Forward => "Next position",
        NavStep::Live => "Current position",
    }
}
