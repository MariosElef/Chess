use std::rc::Rc;

use gpui_kit::{
    assets::IconName,
    component::{
        ActiveTheme as _, Disableable as _, Sizable as _,
        button::{Button, ButtonVariants},
        h_flex,
        popover::{Popover, PopoverState},
        v_flex,
    },
    prelude::FluentBuilder as _,
    *,
};

use super::{avatar::square_avatar, social_pictures::CHALLENGE_PICTURE};
use crate::components::shell::MESSAGES_PICTURE;

type OnClick = Rc<dyn Fn(&mut Window, &mut App)>;

/// How big a friend's picture is.
const PICTURE: Pixels = px(72.);

/// A friend's row: their picture, when they were last online, and Challenge and Message.
#[derive(IntoElement)]
pub struct FriendRow {
    id: SharedString,
    name: SharedString,
    /// Their picture's asset path; none shows their initial.
    picture: Option<String>,
    online: bool,
    seen: SharedString,
    on_challenge: OnClick,
    on_message: OnClick,
    on_remove: OnClick,
    on_block: OnClick,
}

impl FriendRow {
    pub fn new(
        (id, name): (impl Into<SharedString>, impl Into<SharedString>),
        (online, seen): (bool, impl Into<SharedString>),
        on_challenge: impl Fn(&mut Window, &mut App) + 'static,
        (on_message, on_remove, on_block): (
            impl Fn(&mut Window, &mut App) + 'static,
            impl Fn(&mut Window, &mut App) + 'static,
            impl Fn(&mut Window, &mut App) + 'static,
        ),
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            picture: None,
            online,
            seen: seen.into(),
            on_challenge: Rc::new(on_challenge),
            on_message: Rc::new(on_message),
            on_remove: Rc::new(on_remove),
            on_block: Rc::new(on_block),
        }
    }

    pub fn picture(mut self, picture: Option<String>) -> Self {
        self.picture = picture;
        self
    }
}

impl RenderOnce for FriendRow {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let (muted, green, ground) = (theme.muted_foreground, theme.success, theme.muted);
        let picture = div()
            .relative()
            .child(square_avatar(
                &self.name,
                self.picture.as_deref(),
                PICTURE,
                cx,
            ))
            .when(self.online, |this| {
                this.child(
                    div()
                        .absolute()
                        .right(px(-3.))
                        .bottom(px(-3.))
                        .size_4()
                        .rounded_full()
                        .border_2()
                        .border_color(ground)
                        .bg(green),
                )
            });
        let seen_colour = if self.online { green } else { muted };
        let (challenge, message) = (Rc::clone(&self.on_challenge), Rc::clone(&self.on_message));
        h_flex()
            .w_full()
            .gap_4()
            .py_2()
            .child(picture)
            .child(
                v_flex()
                    .flex_1()
                    .min_w_0()
                    .gap_0p5()
                    .child(
                        div()
                            .text_base()
                            .font_weight(FontWeight::BOLD)
                            .child(self.name.clone()),
                    )
                    .child(div().text_sm().text_color(seen_colour).child(self.seen)),
            )
            .child(
                h_flex()
                    .flex_shrink_0()
                    .gap_1()
                    .child(
                        Button::new(SharedString::from(format!("challenge-{}", self.id)))
                            .ghost()
                            .debug_selector({
                                let selector = format!("challenge-{}", self.id);
                                move || selector.clone()
                            })
                            // A picture does not fade by itself as an icon does: an offline
                            // friend's is faded here, so it does not look like it can be pressed.
                            .child(
                                img(CHALLENGE_PICTURE)
                                    .size_5()
                                    .when(!self.online, |this| this.opacity(0.35)),
                            )
                            .tooltip(if self.online {
                                "Challenge"
                            } else {
                                "Offline: a friend can be challenged once they are online"
                            })
                            .disabled(!self.online)
                            .on_click(move |_, window, cx| challenge(window, cx)),
                    )
                    .child(
                        Button::new(SharedString::from(format!("message-{}", self.id)))
                            .ghost()
                            .child(img(MESSAGES_PICTURE).size_5())
                            .tooltip("Message")
                            .on_click(move |_, window, cx| message(window, cx)),
                    )
                    .child(more_menu(&self.id, (self.on_remove, self.on_block))),
            )
    }
}

/// The "…" menu: Remove friend, and Block.
fn more_menu(id: &SharedString, (on_remove, on_block): (OnClick, OnClick)) -> Popover {
    let trigger = Button::new(SharedString::from(format!("more-{id}")))
        .ghost()
        .icon(IconName::Ellipsis);
    let id = id.clone();
    Popover::new(SharedString::from(format!("more-menu-{id}")))
        .anchor(Anchor::TopRight)
        .trigger(trigger)
        .p_1()
        .content(move |_, _, cx| {
            let menu = cx.entity().downgrade();
            v_flex()
                .items_start()
                .child(menu_item(
                    (
                        format!("unfriend-{id}"),
                        IconName::UserMinus,
                        "Remove friend",
                    ),
                    Rc::clone(&on_remove),
                    menu.clone(),
                ))
                .child(menu_item(
                    (format!("block-{id}"), IconName::Ban, "Block"),
                    Rc::clone(&on_block),
                    menu,
                ))
        })
}

/// One of the menu's choices: it acts, then the menu closes.
fn menu_item(
    (id, icon, label): (String, IconName, &'static str),
    action: OnClick,
    menu: WeakEntity<PopoverState>,
) -> Button {
    Button::new(SharedString::from(id))
        .ghost()
        .small()
        .icon(icon)
        .label(label)
        .on_click(move |_, window, cx| {
            action(window, cx);
            // Gone already only if the window is closing.
            let _ = menu.update(cx, |menu, cx| menu.dismiss(window, cx));
        })
}

#[cfg(test)]
mod tests {
    use std::{cell::Cell, rc::Rc};

    use gpui_kit::{
        Context, IntoElement, ParentElement as _, Render, Styled as _, TestAppContext, Window, div,
    };

    use super::FriendRow;
    use crate::testing::click;

    /// A window with one friend's row; `pressed` counts presses of Challenge.
    struct Row {
        online: bool,
        pressed: Rc<Cell<u32>>,
    }

    impl Render for Row {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let pressed = Rc::clone(&self.pressed);
            div().size_full().child(FriendRow::new(
                ("bob", "bob"),
                (self.online, "Online now"),
                move |_, _| pressed.set(pressed.get() + 1),
                (|_, _| {}, |_, _| {}, |_, _| {}),
            ))
        }
    }

    fn presses(cx: &mut TestAppContext, online: bool) -> u32 {
        cx.update(gpui_kit::init);
        let pressed = Rc::new(Cell::new(0));
        let counted = Rc::clone(&pressed);
        let (_, cx) = cx.add_window_view(move |_, _| Row {
            online,
            pressed: counted,
        });
        click(cx, "challenge-bob");
        pressed.get()
    }

    #[gpui_kit::test]
    fn an_online_friends_challenge_button_takes_the_click(cx: &mut TestAppContext) {
        assert_eq!(presses(cx, true), 1);
    }

    #[gpui_kit::test]
    fn an_offline_friends_challenge_button_does_not(cx: &mut TestAppContext) {
        assert_eq!(presses(cx, false), 0);
    }
}
