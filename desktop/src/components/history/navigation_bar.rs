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

use crate::enums::history::NavStep;

type OnStep = Rc<dyn Fn(NavStep, &mut Window, &mut App)>;
type OnHold = Rc<dyn Fn(Option<NavStep>, &mut Window, &mut App)>;

/// ⏮ ◀ ▶ ⏭ — the buttons at either end disable themselves when there is nowhere to go.
#[derive(IntoElement)]
pub struct NavigationBar {
    can_go_back: bool,
    can_go_forward: bool,
    on_step: OnStep,
    /// Back and Forward held down: `Some` when pressed, `None` when let go.
    on_hold: Option<OnHold>,
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
            on_hold: None,
        }
    }

    /// Holding Back or Forward keeps stepping until let go.
    pub fn on_hold(
        mut self,
        on_hold: impl Fn(Option<NavStep>, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_hold = Some(Rc::new(on_hold));
        self
    }
}

impl RenderOnce for NavigationBar {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        h_flex()
            .gap_2()
            .child(self.button(NavStep::Start, !self.can_go_back))
            .child(self.held(NavStep::Back, !self.can_go_back))
            .child(self.held(NavStep::Forward, !self.can_go_forward))
            .child(self.button(NavStep::Live, !self.can_go_forward))
    }
}

impl NavigationBar {
    /// A button that also repeats while held, when the page asks for that.
    fn held(&self, step: NavStep, disabled: bool) -> Div {
        let button = div()
            .debug_selector(move || format!("held-{}", step_id(step)))
            .child(self.button(step, disabled));
        let Some(on_hold) = self.on_hold.clone().filter(|_| !disabled) else {
            return button;
        };
        let (pressed, released, released_outside) =
            (Rc::clone(&on_hold), Rc::clone(&on_hold), on_hold);
        button
            .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                pressed(Some(step), window, cx);
            })
            .on_mouse_up(MouseButton::Left, move |_, window, cx| {
                released(None, window, cx);
            })
            .on_mouse_up_out(MouseButton::Left, move |_, window, cx| {
                released_outside(None, window, cx);
            })
    }

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
