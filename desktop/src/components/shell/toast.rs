use std::rc::Rc;

use gpui_kit::{
    assets::IconName,
    component::{
        ActiveTheme as _, Sizable as _,
        button::{Button, ButtonVariants},
        h_flex, v_flex,
    },
    prelude::FluentBuilder as _,
    *,
};

use crate::{
    components::{panel::START_GREEN, social::avatar},
    models::account::Face,
};

type OnClick = Rc<dyn Fn(&mut Window, &mut App)>;

/// How wide a toast is, as mc2's.
const WIDTH: Pixels = px(360.);
/// The time-left bar along its bottom.
const BAR: Pixels = px(3.);

/// A corner card as mc2-ui's toast: who, a title, a line, buttons, an X and a time-left bar.
#[derive(IntoElement)]
pub struct Toast {
    id: SharedString,
    person: Option<Face>,
    title: SharedString,
    description: Option<SharedString>,
    /// Why the last answer did not go through, in red.
    problem: Option<SharedString>,
    /// What is left of its time, from 1 down to 0; none for a toast that waits.
    time_left: Option<f32>,
    actions: Vec<AnyElement>,
    on_dismiss: Option<OnClick>,
}

impl Toast {
    pub fn new(id: impl Into<SharedString>, title: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            person: None,
            title: title.into(),
            description: None,
            problem: None,
            time_left: None,
            actions: Vec::new(),
            on_dismiss: None,
        }
    }

    /// The person it is about: their picture, or their initial without one.
    pub fn person(mut self, face: Face) -> Self {
        self.person = Some(face);
        self
    }

    pub fn description(mut self, description: impl Into<SharedString>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn problem(mut self, problem: Option<String>) -> Self {
        self.problem = problem.map(SharedString::from);
        self
    }

    /// The share of the wait left, from 1 down to 0, drawn as a shrinking bar; none draws none.
    pub fn time_left(mut self, fraction: Option<f32>) -> Self {
        self.time_left = fraction;
        self
    }

    pub fn action(mut self, action: impl IntoElement) -> Self {
        self.actions.push(action.into_any_element());
        self
    }

    pub fn on_dismiss(mut self, on_dismiss: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_dismiss = Some(Rc::new(on_dismiss));
        self
    }
}

impl RenderOnce for Toast {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let (muted, danger) = (theme.muted_foreground, theme.danger);
        let words = v_flex()
            .flex_1()
            .min_w_0()
            .gap_1()
            .child(div().font_weight(FontWeight::SEMIBOLD).child(self.title))
            .when_some(self.description, |this, description| {
                this.child(div().text_sm().text_color(muted).child(description))
            })
            .when_some(self.problem, |this, problem| {
                this.child(div().text_sm().text_color(danger).child(problem))
            })
            .when(!self.actions.is_empty(), |this| {
                this.child(h_flex().pt_2().gap_2().children(self.actions))
            });
        let close = self.on_dismiss.map(|on_dismiss| {
            Button::new(SharedString::from(format!("{}-dismiss", self.id)))
                .ghost()
                .xsmall()
                .icon(IconName::Close)
                .on_click(move |_, window, cx| on_dismiss(window, cx))
        });
        div()
            .id(self.id)
            .relative()
            .w(WIDTH)
            .overflow_hidden()
            .rounded(px(8.))
            .border_1()
            .border_color(theme.border)
            .bg(theme.popover)
            .text_color(theme.popover_foreground)
            .shadow_lg()
            .child(
                h_flex()
                    .items_start()
                    .gap_3()
                    .p_4()
                    .when_some(self.person, |this, face| {
                        this.child(avatar(&face.name, face.picture.as_deref(), px(40.), cx))
                    })
                    .child(words)
                    .children(close),
            )
            .when_some(self.time_left, |this, left| {
                this.child(
                    div()
                        .absolute()
                        .bottom_0()
                        .left_0()
                        .h(BAR)
                        .w(bar_width(left))
                        .bg(rgb(START_GREEN)),
                )
            })
    }
}

/// How much of the toast's width the time-left bar spans.
fn bar_width(left: f32) -> Pixels {
    WIDTH * left
}
