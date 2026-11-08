use gpui_kit::{
    component::{ActiveTheme as _, h_flex, label::Label, v_flex},
    prelude::FluentBuilder as _,
    *,
};

/// One player in a list: a dot for online or not, their name, a line about them, and the
/// buttons for what can be done.
#[derive(IntoElement)]
pub struct PersonRow {
    name: SharedString,
    detail: Option<SharedString>,
    online: Option<bool>,
    actions: Vec<AnyElement>,
}

impl PersonRow {
    pub fn new(name: impl Into<SharedString>) -> Self {
        Self {
            name: name.into(),
            detail: None,
            online: None,
            actions: Vec::new(),
        }
    }

    pub fn detail(mut self, detail: impl Into<SharedString>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    /// Shows the dot: green online, grey not.
    pub fn online(mut self, online: bool) -> Self {
        self.online = Some(online);
        self
    }

    pub fn action(mut self, action: impl IntoElement) -> Self {
        self.actions.push(action.into_any_element());
        self
    }
}

impl RenderOnce for PersonRow {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let muted = cx.theme().muted_foreground;
        h_flex()
            .w_full()
            .justify_between()
            .gap_3()
            .px_3()
            .py_2()
            .rounded(cx.theme().radius)
            .border_1()
            .border_color(cx.theme().border)
            .child(
                h_flex()
                    .gap_3()
                    .when_some(self.online, |this, online| {
                        let colour = if online { cx.theme().success } else { muted };
                        this.child(div().size_2().rounded_full().bg(colour))
                    })
                    .child(
                        v_flex().child(Label::new(self.name)).when_some(
                            self.detail,
                            |this, detail| {
                                this.child(div().text_xs().text_color(muted).child(detail))
                            },
                        ),
                    ),
            )
            .child(h_flex().gap_2().children(self.actions))
    }
}
