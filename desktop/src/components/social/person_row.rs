use gpui_kit::{
    component::{ActiveTheme as _, h_flex, v_flex},
    prelude::FluentBuilder as _,
    *,
};

use super::avatar::avatar;

/// How big a person's avatar is in a list.
const AVATAR: Pixels = px(40.);

/// One player in a list: their avatar, their name, a line about them, and their buttons.
#[derive(IntoElement)]
pub struct PersonRow {
    name: SharedString,
    /// Their picture's asset path; none shows their initial.
    picture: Option<String>,
    detail: Option<SharedString>,
    actions: Vec<AnyElement>,
}

impl PersonRow {
    pub fn new(name: impl Into<SharedString>) -> Self {
        Self {
            name: name.into(),
            picture: None,
            detail: None,
            actions: Vec::new(),
        }
    }

    pub fn picture(mut self, picture: Option<String>) -> Self {
        self.picture = picture;
        self
    }

    pub fn detail(mut self, detail: impl Into<SharedString>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    pub fn action(mut self, action: impl IntoElement) -> Self {
        self.actions.push(action.into_any_element());
        self
    }
}

impl RenderOnce for PersonRow {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let (muted, hover) = (theme.muted_foreground, theme.accent);
        h_flex()
            .w_full()
            .justify_between()
            .gap_3()
            .px_3()
            .py_2()
            .rounded(px(6.))
            .hover(move |this| this.bg(hover))
            .child(
                h_flex()
                    .min_w_0()
                    .gap_3()
                    .child(avatar(&self.name, self.picture.as_deref(), AVATAR, cx))
                    .child(
                        v_flex()
                            .min_w_0()
                            .child(div().font_weight(FontWeight::SEMIBOLD).child(self.name))
                            .when_some(self.detail, |this, detail| {
                                this.child(div().text_xs().text_color(muted).child(detail))
                            }),
                    ),
            )
            .child(h_flex().flex_shrink_0().gap_1().children(self.actions))
    }
}
